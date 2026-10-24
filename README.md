# Atelier Roadmap

Aplicativo de agenda e direção artística para **Windows 10/11 x64**, desenvolvido em **Rust**. Janela própria com WebView2 (Wry/Tao), sem abrir o navegador.

## Instalar

Baixe **Atelier-Setup.exe** na [última release](https://github.com/JDantanna/atelier-roadmap/releases/latest) e execute. O instalador prepara o WebView2 da Microsoft se necessário (essa etapa exige internet), cria o atalho e instala para o usuário atual, sem exigir Rust.

## Atualizar pelo aplicativo

Em **Ajustes → Checar atualização**, o aplicativo consulta exclusivamente as releases de `JDantanna/atelier-roadmap`. Quando houver uma versão mais nova, escolha **Baixar atualização** e depois **Instalar e reiniciar**. As versões de teste e os downgrades são rejeitados. O download é verificado por SHA-256 antes da instalação e novamente antes da execução.

Dados ficam separados da instalação, em `%LOCALAPPDATA%\AtelierRoadmap\agenda.json`; a versão anterior fica em `agenda.backup.json`. Atualizar ou desinstalar não apaga a agenda. A interface também oferece exportação e restauração de backup.

## Atualizar o plano do mentor

O arquivo [`mentor/plan.json`](mentor/plan.json) contém o plano compartilhável. Quando uma revisão do chat resultar em mudanças, editamos esse arquivo no GitHub. Depois use **Ajustes → Buscar plano do mentor**: leia a prévia e aplique. Não exige reinstalação nem uma nova release.

- Não existe leitura automática das conversas do ChatGPT.
- Nenhum registro pessoal, receita, anotação ou progresso local é enviado ao GitHub.
- Alterações por ID preservam tempo real, notas e conclusão. IDs novos criam tarefas; atualizações não apagam tarefas.
- Cada revisão deve usar um `updateId` único. O mesmo plano não é aplicado duas vezes.
- Para compartilhar progresso com o mentor, exporte o resumo e envie no chat por decisão própria. Não faça commit desse resumo no repositório público.

## Agenda inicial

Seis horas de segunda a sexta, início às 09h: estudo aplicado (45min), produção (4h), prospecção (45min) e revisão (30min). Horários, dias, capacidade, tarefas e metas são editáveis. Os intervalos podem ser inseridos remarcando os blocos.

Telas: Hoje, Semana, Mês, Roadmap, Resultados e Ajustes. Há cronômetro, registro de tempo, conclusão e reabertura de tarefas, edição de metas e controle de recebimentos. O modelo inicial cobre outubro de 2026 a março de 2027; é uma estrutura de trabalho editável, e não uma promessa de resultado financeiro.

Pause o cronômetro antes de suspender o computador. Ele mede tempo de relógio, faz checkpoints a cada 15 segundos e pode incluir suspensão. Ao fechar a janela, o aplicativo pausa e salva antes de encerrar. A agenda também pode ser restaurada do JSON exportado pela versão portátil 0.1.

## Desenvolvimento

Pré-requisitos no Windows: Rust estável com MSVC, ferramentas C++ e Microsoft WebView2 Runtime.

```text
cargo test --locked
cargo run
cargo build --release --locked
```

Para testar a interface no Linux, use `ATELIER_HEADLESS=1 cargo run`. A janela nativa é exclusiva do Windows. O servidor local aceita somente 127.0.0.1, tem rota aleatória por sessão, valida a origem e não oferece acesso arbitrário a arquivos.

Testes da interface:

```text
cargo build --locked
npm ci
npx playwright install chromium
python tests/run-ui.py
```

## Publicação e próximas versões

O workflow [Windows — testar, compilar e publicar](.github/workflows/windows-release.yml) testa o núcleo, a interface e persistência, compila o programa e gera o instalador Inno Setup. Valida a assinatura do bootstrapper WebView2 da Microsoft e testa a instalação silenciosa sem alterar os dados existentes. A release só é publicada depois desses passos passarem.

1. Faça a mudança de código e atualize testes relevantes.
2. Aumente a versão em `Cargo.toml`, `package.json`, `package-lock.json` e na indicação de versão da interface. Atualize o lock do Cargo.
3. Faça commit no `main`. O workflow publica uma release `vX.Y.Z` com `Atelier-Setup.exe`, checksum e executável.
4. O usuário recebe essa versão ao checar atualizações no aplicativo.

Um commit só em `mentor/` não recompila o aplicativo: o plano é baixado diretamente pelo botão. Alterações apenas de documentação também não disparam release. O workflow pode ser executado manualmente em Actions. Releases existentes não são sobrescritas.

O instalador ainda não possui assinatura digital do autor. O checksum protege contra downloads incompletos e substituições acidentais; a confiança na origem depende do HTTPS e do controle deste repositório.
