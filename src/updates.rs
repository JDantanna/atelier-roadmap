use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::Duration,
};
const REPO: &str = "JDantanna/atelier-roadmap";
const VERSION: &str = env!("CARGO_PKG_VERSION");
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Release {
    pub version: String,
    pub url: String,
    pub checksum_url: String,
}
#[derive(Clone, Serialize)]
pub struct Status {
    pub phase: String,
    pub version: String,
    pub message: String,
    pub percent: u8,
    pub release: Option<Release>,
    #[serde(skip)]
    pub installer: Option<PathBuf>,
    #[serde(skip)]
    pub checksum: Option<String>,
}
pub type Shared = Arc<Mutex<Status>>;
pub fn new() -> Shared {
    Arc::new(Mutex::new(Status {
        phase: "idle".into(),
        version: VERSION.into(),
        message: "Consulte novas versões no GitHub.".into(),
        percent: 0,
        release: None,
        installer: None,
        checksum: None,
    }))
}
fn client() -> Result<Client, String> {
    Client::builder()
        .user_agent(concat!("Atelier/", env!("CARGO_PKG_VERSION")))
        .timeout(Duration::from_secs(180))
        .connect_timeout(Duration::from_secs(12))
        .https_only(true)
        .build()
        .map_err(|e| e.to_string())
}
fn asset_url(s: &str) -> bool {
    let Ok(u) = reqwest::Url::parse(s) else {
        return false;
    };
    u.scheme() == "https"
        && u.host_str() == Some("github.com")
        && u.username().is_empty()
        && u.password().is_none()
        && u.port_or_known_default() == Some(443)
        && u.query().is_none()
        && u.fragment().is_none()
        && u.path().starts_with(&format!("/{REPO}/releases/download/"))
}
pub fn parse_release(data: &serde_json::Value, current: &str) -> Result<Option<Release>, String> {
    if data["draft"].as_bool().unwrap_or(true) || data["prerelease"].as_bool().unwrap_or(true) {
        return Ok(None);
    }
    let version = data["tag_name"]
        .as_str()
        .ok_or("Versão ausente")?
        .trim_start_matches('v');
    let next = semver::Version::parse(version).map_err(|_| "Versão inválida")?;
    let current = semver::Version::parse(current).map_err(|_| "Versão atual inválida")?;
    if next <= current || !next.pre.is_empty() {
        return Ok(None);
    }
    let assets = data["assets"].as_array().ok_or("Release sem arquivos")?;
    let url = |name| -> Result<String, String> {
        let s = assets
            .iter()
            .find(|a| a["name"].as_str() == Some(name))
            .and_then(|a| a["browser_download_url"].as_str())
            .ok_or("O instalador desta versão ainda não está disponível")?;
        if !asset_url(s) {
            return Err("Origem do instalador inválida".into());
        }
        Ok(s.into())
    };
    Ok(Some(Release {
        version: version.into(),
        url: url("Atelier-Setup.exe")?,
        checksum_url: url("Atelier-Setup.exe.sha256")?,
    }))
}
pub fn check(shared: Shared) {
    {
        let mut s = shared.lock().unwrap();
        if ["checking", "downloading", "ready", "installing"].contains(&s.phase.as_str()) {
            return;
        }
        s.phase = "checking".into();
        s.message = "Consultando o GitHub…".into();
    }
    std::thread::spawn(move || {
        let result = (|| {
            let response = client()?
                .get(format!(
                    "https://api.github.com/repos/{REPO}/releases/latest"
                ))
                .timeout(Duration::from_secs(20))
                .send()
                .map_err(|_| "Sem conexão com o GitHub")?;
            if response.status() == 404 {
                return Ok(None);
            }
            let data = response
                .error_for_status()
                .map_err(|_| "O GitHub recusou a consulta. Tente novamente mais tarde.")?
                .json::<serde_json::Value>()
                .map_err(|_| "Resposta inválida")?;
            parse_release(&data, VERSION)
        })();
        let mut s = shared.lock().unwrap();
        match result {
            Ok(Some(r)) => {
                s.message = format!("Versão {} disponível.", r.version);
                s.release = Some(r);
                s.phase = "available".into()
            }
            Ok(None) => {
                s.phase = "current".into();
                s.message = format!("Você está na versão mais recente disponível: {VERSION}.")
            }
            Err(e) => {
                s.phase = "error".into();
                s.message = e;
            }
        }
    });
}
pub fn checksum_text(text: &str) -> Result<String, String> {
    let hash = text.split_whitespace().next().ok_or("Checksum ausente")?;
    if hash.len() != 64 || !hash.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("Checksum inválido".into());
    }
    Ok(hash.to_lowercase())
}
pub fn verify(path: &Path, hash: &str) -> Result<(), String> {
    let mut f = fs::File::open(path).map_err(|e| e.to_string())?;
    let mut h = Sha256::new();
    let mut b = [0; 65536];
    loop {
        let n = f.read(&mut b).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        h.update(&b[..n]);
    }
    if format!("{:x}", h.finalize()) != hash {
        return Err("A verificação do arquivo falhou. Baixe novamente.".into());
    }
    Ok(())
}
pub fn download(shared: Shared, directory: PathBuf) {
    let release = {
        let mut s = shared.lock().unwrap();
        if s.phase != "available" {
            return;
        }
        let Some(r) = s.release.clone() else { return };
        s.phase = "downloading".into();
        s.percent = 0;
        s.message = "Baixando atualização. Pode continuar usando a agenda.".into();
        r
    };
    std::thread::spawn(move || {
        let result = (|| -> Result<(PathBuf, String), String> {
            if !asset_url(&release.url) || !asset_url(&release.checksum_url) {
                return Err("Origem inválida".into());
            }
            let client = client()?;
            let mut response = client
                .get(&release.checksum_url)
                .send()
                .map_err(|e| e.to_string())?
                .error_for_status()
                .map_err(|e| e.to_string())?;
            let mut bytes = Vec::new();
            response
                .by_ref()
                .take(4097)
                .read_to_end(&mut bytes)
                .map_err(|e| e.to_string())?;
            if bytes.len() > 4096 {
                return Err("Checksum grande demais".into());
            }
            let checksum =
                checksum_text(&String::from_utf8(bytes).map_err(|_| "Checksum inválido")?)?;
            fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
            let path = directory.join(format!("Atelier-Setup-{}.exe", release.version));
            let tmp = path.with_extension("part");
            let mut response = client
                .get(&release.url)
                .send()
                .map_err(|e| e.to_string())?
                .error_for_status()
                .map_err(|e| e.to_string())?;
            let total = response.content_length().unwrap_or(0);
            if total > 250 * 1024 * 1024 {
                return Err("Instalador grande demais".into());
            }
            let mut out = fs::File::create(&tmp).map_err(|e| e.to_string())?;
            let mut count = 0u64;
            let mut buf = [0u8; 65536];
            loop {
                let n = response.read(&mut buf).map_err(|e| e.to_string())?;
                if n == 0 {
                    break;
                }
                count += n as u64;
                if count > 250 * 1024 * 1024 {
                    return Err("Limite de download excedido".into());
                }
                out.write_all(&buf[..n]).map_err(|e| e.to_string())?;
                if total > 0 {
                    shared.lock().unwrap().percent = ((count * 100 / total).min(99)) as u8;
                }
            }
            out.sync_all().map_err(|e| e.to_string())?;
            drop(out);
            verify(&tmp, &checksum)?;
            if path.exists() {
                fs::remove_file(&path).map_err(|e| e.to_string())?
            }
            fs::rename(&tmp, &path).map_err(|e| e.to_string())?;
            Ok((path, checksum))
        })();
        let mut s = shared.lock().unwrap();
        match result {
            Ok((path, checksum)) => {
                s.installer = Some(path);
                s.checksum = Some(checksum);
                s.phase = "ready".into();
                s.percent = 100;
                s.message="Pronto. Salve seus formulários e clique em Instalar e reiniciar. Seus dados serão preservados.".into()
            }
            Err(e) => {
                s.phase = "available".into();
                s.message = format!("Download não concluído: {e}");
            }
        }
    });
}
pub fn install(shared: &Shared) -> Result<(), String> {
    let mut s = shared.lock().unwrap();
    if s.phase != "ready" {
        return Err("Atualização ainda não está pronta".into());
    }
    let path = s.installer.as_ref().ok_or("Instalador ausente")?;
    verify(path, s.checksum.as_deref().ok_or("Checksum ausente")?)?;
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        let parent = exe.parent().ok_or("Diretório inválido")?;
        std::process::Command::new(path)
            .args([
                "/VERYSILENT",
                "/SUPPRESSMSGBOXES",
                "/NORESTART",
                "/CLOSEAPPLICATIONS",
            ])
            .arg(format!("/DIR={}", parent.display()))
            .creation_flags(0x08000000)
            .spawn()
            .map_err(|e| format!("Não foi possível iniciar o instalador: {e}"))?;
        s.phase = "installing".into();
        s.message = "Instalando e reiniciando…".into();
        Ok(())
    }
    #[cfg(not(windows))]
    {
        s.message = "A instalação automática é exclusiva do Windows.".into();
        Err(s.message.clone())
    }
}
pub fn mentor_plan() -> Result<serde_json::Value, String> {
    let response = client()?
        .get(format!(
            "https://raw.githubusercontent.com/{REPO}/main/mentor/plan.json"
        ))
        .header("Cache-Control", "no-cache")
        .timeout(Duration::from_secs(20))
        .send()
        .map_err(|_| "Não foi possível consultar o plano no GitHub.")?
        .error_for_status()
        .map_err(|_| "Plano indisponível no GitHub.")?;
    let mut bytes = Vec::new();
    response
        .take(4 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > 4 * 1024 * 1024 {
        return Err("Plano grande demais".into());
    }
    serde_json::from_slice(&bytes).map_err(|_| "O plano do GitHub não é um JSON válido.".into())
}
#[cfg(test)]
mod tests {
    use super::*;
    fn release(v: &str) -> serde_json::Value {
        serde_json::json!({"draft":false,"prerelease":false,"tag_name":v,"assets":[{"name":"Atelier-Setup.exe","browser_download_url":"https://github.com/JDantanna/atelier-roadmap/releases/download/v0.3.0/Atelier-Setup.exe"},{"name":"Atelier-Setup.exe.sha256","browser_download_url":"https://github.com/JDantanna/atelier-roadmap/releases/download/v0.3.0/Atelier-Setup.exe.sha256"}]})
    }
    #[test]
    fn newer_only() {
        assert!(parse_release(&release("v0.3.0"), "0.2.0")
            .unwrap()
            .is_some());
        assert!(parse_release(&release("v0.1.0"), "0.2.0")
            .unwrap()
            .is_none());
        assert!(parse_release(&release("v0.2.0"), "0.2.0")
            .unwrap()
            .is_none());
    }
    #[test]
    fn prerelease_rejected() {
        assert!(parse_release(&release("v0.3.0-beta.1"), "0.2.0")
            .unwrap()
            .is_none());
    }
    #[test]
    fn foreign_assets_rejected() {
        let mut r = release("v0.3.0");
        r["assets"][0]["browser_download_url"] = "https://example.com/setup.exe".into();
        assert!(parse_release(&r, "0.2.0").is_err());
        assert!(!asset_url(
            "https://github.com@evil.test/JDantanna/atelier-roadmap/releases/download/v1/x"
        ));
    }
    #[test]
    fn bad_checksum_rejected() {
        assert!(checksum_text("xyz").is_err());
        assert!(checksum_text(&format!("{}  file.exe", "a".repeat(64))).is_ok());
    }
    #[test]
    fn changed_bytes_rejected() {
        let p = std::env::temp_dir().join(format!("atelier-hash-{}.tmp", std::process::id()));
        fs::write(&p, b"original").unwrap();
        let hash = format!("{:x}", Sha256::digest(b"original"));
        assert!(verify(&p, &hash).is_ok());
        fs::write(&p, b"changed").unwrap();
        assert!(verify(&p, &hash).is_err());
        let _ = fs::remove_file(p);
    }
}
