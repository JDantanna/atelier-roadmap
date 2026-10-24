use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    path::PathBuf,
    sync::{mpsc::Sender, Arc, Mutex},
    time::Duration,
};
#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Notice {
    pub id: String,
    pub task_id: Option<String>,
    pub title: String,
    pub body: String,
    pub created_at: i64,
    pub read: bool,
    pub snoozed_until: Option<i64>,
}
#[derive(Clone, Default, Serialize, Deserialize)]
struct Ledger {
    notices: Vec<Notice>,
    fired: HashSet<String>,
}
struct Engine {
    path: PathBuf,
    ledger: Ledger,
}
#[derive(Clone)]
pub struct Hub(Arc<Mutex<Engine>>, Sender<Notice>);
fn due(task: &serde_json::Value, now: i64) -> Option<(String, String, String)> {
    if task["status"].as_str() == Some("done") {
        return None;
    }
    let iso = task["dueAt"].as_str()?;
    let deadline = DateTime::parse_from_rfc3339(iso).ok()?.timestamp_millis();
    let minutes = task["remindMinutes"].as_i64().unwrap_or(30);
    if minutes < 0 {
        return None;
    }
    let title = task["title"].as_str().unwrap_or("Tarefa");
    let id = task["id"].as_str()?;
    let (kind, label) = if now >= deadline {
        (
            "due",
            if now - deadline > 300000 {
                "Prazo vencido"
            } else {
                "Prazo atingido"
            },
        )
    } else if minutes > 0 && now >= deadline - minutes * 60000 {
        ("before", "Prazo se aproximando")
    } else {
        return None;
    };
    Some((
        format!("{id}:{iso}:{kind}:{minutes}"),
        format!("{label}: {title}"),
        format!(
            "{} • Prazo: {}",
            task["project"].as_str().unwrap_or("Projeto"),
            DateTime::parse_from_rfc3339(iso)
                .ok()?
                .with_timezone(&chrono::Local)
                .format("%d/%m/%Y %H:%M")
        ),
    ))
}
fn active_task(t: &serde_json::Value, state: &serde_json::Value) -> bool {
    t["status"].as_str() != Some("done")
        && t["remindMinutes"].as_i64().unwrap_or(30) >= 0
        && !state["projects"]
            .as_array()
            .map(|ps| {
                ps.iter()
                    .any(|p| p["id"] == t["projectId"] && p["archived"].as_bool() == Some(true))
            })
            .unwrap_or(false)
}
impl Hub {
    pub fn new(path: PathBuf, sender: Sender<Notice>) -> Result<Self, String> {
        let ledger = if path.exists() {
            serde_json::from_slice(&std::fs::read(&path).map_err(|e| e.to_string())?)
                .map_err(|e| format!("Histórico de avisos inválido: {e}"))?
        } else {
            Ledger::default()
        };
        Ok(Self(Arc::new(Mutex::new(Engine { path, ledger })), sender))
    }
    pub fn list(&self) -> Vec<Notice> {
        self.0.lock().unwrap().ledger.notices.clone()
    }
    fn save(e: &Engine) -> Result<(), String> {
        crate::save(
            &e.path,
            &serde_json::to_vec(&e.ledger).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())
    }
    pub fn tick(&self, state: &serde_json::Value, now: i64) -> Result<(), String> {
        if state["settings"]["notificationsEnabled"].as_bool() == Some(false) {
            return Ok(());
        }
        let mut engine = self.0.lock().unwrap();
        let previous = engine.ledger.clone();
        let mut send = Vec::new();
        let mut changed = false;
        for n in &mut engine.ledger.notices {
            if n.snoozed_until.is_some()
                && n.task_id
                    .as_ref()
                    .map(|id| {
                        !state["tasks"]
                            .as_array()
                            .map(|ts| {
                                ts.iter().any(|t| {
                                    t["id"].as_str() == Some(id.as_str()) && active_task(t, state)
                                })
                            })
                            .unwrap_or(false)
                    })
                    .unwrap_or(false)
            {
                n.read = true;
                n.snoozed_until = None;
                changed = true;
            }
            if n.snoozed_until.map(|time| now >= time).unwrap_or(false) && !n.read {
                n.snoozed_until = None;
                send.push(n.clone());
            }
        }
        if let Some(tasks) = state["tasks"].as_array() {
            for t in tasks {
                if !active_task(t, state) {
                    continue;
                }
                if let Some((key, title, body)) = due(t, now) {
                    if engine.ledger.fired.insert(key.clone()) {
                        let n = Notice {
                            id: key,
                            task_id: t["id"].as_str().map(String::from),
                            title,
                            body,
                            created_at: now,
                            read: false,
                            snoozed_until: None,
                        };
                        engine.ledger.notices.push(n.clone());
                        send.push(n);
                    }
                }
            }
        }
        if changed || !send.is_empty() {
            let excess = engine.ledger.notices.len().saturating_sub(500);
            engine.ledger.notices.drain(..excess);
            if let Err(error) = Self::save(&engine) {
                engine.ledger = previous;
                return Err(error);
            }
        }
        drop(engine);
        for n in send {
            let _ = self.1.send(n);
        }
        Ok(())
    }
    pub fn action(&self, id: &str, action: &str, minutes: i64, now: i64) -> Result<(), String> {
        let mut e = self.0.lock().unwrap();
        let previous = e.ledger.clone();
        let n = e
            .ledger
            .notices
            .iter_mut()
            .find(|n| n.id == id)
            .ok_or("Aviso não encontrado")?;
        match action {
            "read" => {
                n.read = true;
                n.snoozed_until = None
            }
            "snooze" => {
                if ![5, 10, 30, 60].contains(&minutes) {
                    return Err("Intervalo inválido".into());
                }
                n.read = false;
                n.snoozed_until = Some(now + minutes * 60000)
            }
            _ => return Err("Ação inválida".into()),
        }
        if let Err(error) = Self::save(&e) {
            e.ledger = previous;
            return Err(error);
        }
        Ok(())
    }
    pub fn test(&self) -> Result<(), String> {
        let now = Utc::now().timestamp_millis();
        let n=Notice{id:format!("test-{now}"),task_id:None,title:"Atelier — teste de lembrete".into(),body:"Os avisos estão ativos. Seus prazos também aparecem na central de notificações do Atelier.".into(),created_at:now,read:false,snoozed_until:None};
        let mut e = self.0.lock().unwrap();
        let previous = e.ledger.clone();
        e.ledger.notices.push(n.clone());
        if let Err(error) = Self::save(&e) {
            e.ledger = previous;
            return Err(error);
        }
        drop(e);
        let _ = self.1.send(n);
        Ok(())
    }
    pub fn start(&self, agenda: PathBuf) {
        let hub = self.clone();
        std::thread::spawn(move || loop {
            if let Ok(bytes) = std::fs::read(&agenda) {
                if let Ok(state) = serde_json::from_slice(&bytes) {
                    let _ = hub.tick(&state, Utc::now().timestamp_millis());
                }
            }
            std::thread::sleep(Duration::from_secs(5));
        });
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn state() -> serde_json::Value {
        json!({"settings":{"notificationsEnabled":true},"tasks":[{"id":"t1","title":"Entrega","project":"Projeto","status":"pending","dueAt":"2026-10-24T15:00:00Z","remindMinutes":30}]})
    }
    fn fixture() -> (Hub, std::sync::mpsc::Receiver<Notice>, PathBuf) {
        let (tx, rx) = std::sync::mpsc::channel();
        let p = std::env::temp_dir().join(format!("atelier-notify-{}.json", crate::token()));
        (Hub::new(p.clone(), tx).unwrap(), rx, p)
    }
    #[test]
    fn storage_failure_retries_without_losing_notification() {
        let (tx, rx) = std::sync::mpsc::channel();
        let dir = std::env::temp_dir().join(crate::token());
        let hub = Hub::new(dir.join("notices.json"), tx).unwrap();
        let time = DateTime::parse_from_rfc3339("2026-10-24T15:00:00Z")
            .unwrap()
            .timestamp_millis();
        assert!(hub.tick(&state(), time).is_err());
        assert!(rx.try_recv().is_err());
        std::fs::create_dir_all(&dir).unwrap();
        hub.tick(&state(), time).unwrap();
        assert!(rx.try_recv().is_ok());
        let _ = std::fs::remove_dir_all(dir);
    }
    #[test]
    fn archive_cancels_pending_snooze() {
        let (h, rx, p) = fixture();
        let time = DateTime::parse_from_rfc3339("2026-10-24T15:00:00Z")
            .unwrap()
            .timestamp_millis();
        let mut s = state();
        s["projects"] = json!([{"id":"p","archived":false}]);
        s["tasks"][0]["projectId"] = "p".into();
        h.tick(&s, time).unwrap();
        let n = rx.recv().unwrap();
        h.action(&n.id, "snooze", 10, time).unwrap();
        s["projects"][0]["archived"] = true.into();
        h.tick(&s, time + 600000).unwrap();
        assert!(rx.try_recv().is_err());
        assert!(h.list()[0].read);
        let _ = std::fs::remove_file(p);
    }
    #[test]
    fn reminder_due_catchup_and_dedup() {
        let (h, rx, p) = fixture();
        let deadline = DateTime::parse_from_rfc3339("2026-10-24T15:00:00Z")
            .unwrap()
            .timestamp_millis();
        h.tick(&state(), deadline - 1800001).unwrap();
        assert!(rx.try_recv().is_err());
        h.tick(&state(), deadline - 1800000).unwrap();
        assert!(rx.try_recv().unwrap().title.contains("aproximando"));
        h.tick(&state(), deadline - 1000).unwrap();
        assert!(rx.try_recv().is_err());
        h.tick(&state(), deadline + 600000).unwrap();
        assert!(rx.try_recv().unwrap().title.contains("vencido"));
        let (tx, rx2) = std::sync::mpsc::channel();
        let reopened = Hub::new(p.clone(), tx).unwrap();
        reopened.tick(&state(), deadline + 900000).unwrap();
        assert!(rx2.try_recv().is_err());
        let _ = std::fs::remove_file(p);
    }
    #[test]
    fn completed_disabled_and_no_deadline_are_silent() {
        let (h, rx, p) = fixture();
        let mut s = state();
        s["tasks"][0]["status"] = "done".into();
        h.tick(&s, i64::MAX / 2).unwrap();
        s["tasks"][0]["status"] = "pending".into();
        s["settings"]["notificationsEnabled"] = false.into();
        h.tick(&s, i64::MAX / 2).unwrap();
        s["settings"]["notificationsEnabled"] = true.into();
        s["tasks"][0]["dueAt"] = serde_json::Value::Null;
        h.tick(&s, i64::MAX / 2).unwrap();
        assert!(rx.try_recv().is_err());
        let _ = std::fs::remove_file(p);
    }
    #[test]
    fn snooze_and_read_persist() {
        let (h, rx, p) = fixture();
        let time = DateTime::parse_from_rfc3339("2026-10-24T15:00:00Z")
            .unwrap()
            .timestamp_millis();
        h.tick(&state(), time).unwrap();
        let n = rx.recv().unwrap();
        h.action(&n.id, "snooze", 10, time).unwrap();
        h.tick(&state(), time + 599999).unwrap();
        assert!(rx.try_recv().is_err());
        h.tick(&state(), time + 600000).unwrap();
        assert_eq!(rx.recv().unwrap().id, n.id);
        h.action(&n.id, "read", 0, time + 600001).unwrap();
        h.tick(&state(), time + 900000).unwrap();
        assert!(rx.try_recv().is_err());
        assert!(h.list()[0].read);
        let _ = std::fs::remove_file(p);
    }
}
