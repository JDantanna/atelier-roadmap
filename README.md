# Atelier 0.4 — ideias e projetos

Um espaço local para pensar visualmente e organizar o próximo passo. Windows 10/11 x64, Rust com WebView2.

## Quadro infinito

A tela inicial é **Quadro**: adicione notas, texto e imagens PNG/JPG/WebP (botão ou arraste um arquivo). Duplo clique edita o item; arraste para mover. Use Mão ou Espaço + arraste para navegar e a roda ou +/− para zoom. Enquadrar reúne os itens. Conectar liga o primeiro item ao segundo. Shift + clique seleciona vários; Excluir remove a seleção e conexões associadas. Desfazer/Refazer também aceita Ctrl+Z, Ctrl+Shift+Z e Ctrl+Y.

Quadro e câmera são salvos automaticamente na agenda local. Imagens são incorporadas, reduzidas para até 1800 px e comprimidas; não dependem do arquivo original e não alteram arquivos externos. Limite do espaço: cerca de 30 MB; cada importação aceita até 15 MB antes da redução. Guarde originais fora do aplicativo para impressão. O histórico de desfazer dura a sessão. Não há colaboração online ou paridade completa com Miro.

## Organização simples

A barra principal contém Quadro, Tarefas e Projetos. Calendário, roadmap, Kanban, notificações e relatórios ficam em **Mais visões**. A tarefa começa com os campos essenciais; prioridade, repetição, lembretes e dependências ficam em **Mais opções**. Prazos continuam funcionando com o aplicativo aberto ou na bandeja.

## Limpar e recuperar projetos

A atualização preserva os dados instalados. Para remover **todos** os projetos, tarefas (inclusive sem projeto) e marcos no seu Windows: **Ajustes → Limpar todos os projetos**. Antes de remover, o aplicativo salva um backup completo e independente em `%LOCALAPPDATA%\AtelierRoadmap\before-cleanup-DATA.json`. Se o backup falhar, nada é removido. A limpeza mantém quadro, preferências e recebimentos; zera o histórico de projetos e bloqueia novas importações do mentor. Não toca em arquivos pessoais externos.

**Recuperar última limpeza** restaura projetos, tarefas e marcos com tempo, notas e conclusão. Mantém o quadro e as preferências atuais. Antes de recuperar cria outra cópia, permitindo voltar ao estado anterior. Para recuperar uma cópia mais antiga, use Restaurar backup com o arquivo before-cleanup correspondente (restauração completa, inclusive quadro). Os backups independentes não são sobrescritos pelas gravações normais.

Novas instalações começam vazias. Não há criação automática de projetos demonstrativos ou do mentor. Após limpar, use **Permitir importar planos novamente** somente quando quiser voltar a importar. A importação sempre mostra uma prévia.

## Atualização e validação

**Ajustes → Checar atualização → Baixar atualização → Instalar e reiniciar**. Alternativa: Atelier-Setup.exe na release do GitHub. Os dados locais são preservados.

Desenvolvimento: `cargo test --locked`, `cargo build --locked`, `npm ci`, `npx playwright install chromium`, `npm test`. O fluxo Windows testa instalação, janela, bandeja e persistência antes de publicar instalador e SHA-256. Testes do quadro cobrem operações reais de ponteiro, importação/reabertura da imagem, edição, limpeza com falha de backup e recuperação sem perder alterações posteriores do quadro.

---

## Notas de versões anteriores

# Atelier — projetos e roadmaps

Gerenciador pessoal de projetos para **Windows 10/11 x64**, feito em **Rust**, com janela própria. A versão 0.3 amplia o antigo planejador artístico; a agenda anterior é preservada.

## Instalar e atualizar

Baixe **Atelier-Setup.exe** na [última versão](https://github.com/JDantanna/atelier-roadmap/releases/latest). Não precisa instalar Rust. O instalador prepara o Microsoft WebView2 quando necessário.

No aplicativo: **Ajustes → Checar atualização → Baixar atualização → Instalar e reiniciar**. A atualização verifica o checksum SHA-256 e mantém os dados separados da instalação. Novas versões são publicadas automaticamente pelo GitHub Actions depois dos testes.

## O que está incluído

- **Projetos:** nome, objetivo, cor, início, prazo final, status, progresso e arquivamento sem apagar o histórico.
- **Tarefas:** quatro status (a fazer, em andamento, bloqueada, concluída), quatro prioridades, etiquetas, notas, estimativa e tempo real.
- **Subtarefas e dependências:** checklist, bloqueio de início/conclusão enquanto houver dependências pendentes e prevenção de ciclos.
- **Recorrência:** ao concluir, cria a próxima ocorrência diária, semanal ou mensal. Reabrir e concluir novamente não duplica a próxima tarefa.
- **Lista, Kanban e busca:** filtro por projeto, prioridade e status; busca por texto; movimentação de cartões e seletor de status acessível.
- **Roadmap:** linha do tempo dos projetos, marcos editáveis e tarefas vinculadas a cada marco.
- **Hoje, semana e mês:** organização do trabalho planejado, separada do prazo de entrega.
- **Prazos de tarefas:** data e hora, lembrete antecipado e aviso no vencimento. Prazos vencidos são recuperados quando o programa volta a abrir.
- **Notificações:** balões nativos do Windows e central interna com histórico, leitura e adiamento. Avisos não são repetidos continuamente; conclusão e arquivamento interrompem os novos avisos.
- **Relatórios:** progresso por projeto, tarefas atrasadas, horas e histórico de alterações.

A edição inicial é para **um usuário**, sem contas, colaboração ou sincronização de dados pessoais com serviços externos.

## Minimizar, bandeja e fechar

Por padrão, o Atelier pergunta:

- Ao minimizar: **Barra de tarefas**, **Continuar na bandeja** ou **Cancelar**.
- Ao fechar: **Salvar e sair**, **Continuar na bandeja** ou **Cancelar**.

A pergunta tem **Lembrar minha escolha**. As preferências podem ser alteradas em **Ajustes → Janela e notificações**. O formulário de tarefa que estiver aberto permanece intacto ao cancelar ou continuar em segundo plano; ao sair, formulários ainda não salvos são descartados.

O ícone da bandeja, junto ao relógio, reabre o programa com um clique. O menu de contexto oferece **Abrir Atelier** e **Salvar e sair**. Se a bandeja não estiver disponível no Windows, o aplicativo permanece na barra de tarefas para que possa ser reaberto.

**Os lembretes exigem que o Atelier continue aberto ou na bandeja.** Sair completamente encerra o programa, sem instalar um serviço ou agendamento oculto. Ao reabrir, o programa verifica os prazos vencidos. O Windows pode silenciar balões conforme suas preferências de notificações/Não incomodar. A central interna conserva os avisos independentemente disso. Use **Testar notificação do Windows** nos Ajustes para conferir a entrega no computador.

## Dados e migração

Dados locais: `%LOCALAPPDATA%\AtelierRoadmap\agenda.json`. A versão anterior de cada gravação fica em `agenda.backup.json`. Os avisos e o controle para evitar notificações duplicadas ficam em `notifications.json`.

Ao abrir uma agenda 0.2, o aplicativo cria os projetos a partir dos agrupamentos existentes e mantém IDs, tarefas, horários, conclusões, notas, tempo, marcos e recebimentos anteriores. Tarefas antigas **não recebem prazos de notificação automaticamente**, evitando avisos inesperados para toda a agenda antiga. Os recebimentos anteriores continuam acessíveis em Relatórios.

Novas instalações começam com um espaço vazio para projetos, sem preencher meses com tarefas artísticas. A capacidade diária é uma preferência editável, não um limite de projetos.

Use **Exportar backup** e **Restaurar backup** para a agenda, projetos e preferências. O histórico de notificações também pode ser preservado copiando `notifications.json` com o programa encerrado. O instalador e o desinstalador não apagam a pasta de dados pessoais.

## Plano pelo GitHub e acompanhamento no chat

A integração opcional em **Ajustes → Buscar plano do mentor** continua disponível. O aplicativo lê [`mentor/plan.json`](mentor/plan.json), mostra a prévia e só aplica após a escolha do usuário. Reutilizar IDs preserva conclusão, notas e tempo; IDs novos acrescentam tarefas. Use um `updateId` único por revisão.

O programa não lê conversas do ChatGPT automaticamente. O código e o plano compartilhável ficam neste repositório público; progresso, notas, receita e dados locais não são enviados. Para revisão no chat, exporte o resumo e envie por decisão própria. Não publique esse resumo no GitHub.

## Desenvolvimento e validação

No Windows: Rust estável/MSVC, ferramentas C++ e WebView2.

```text
cargo test --locked
cargo run
cargo build --release --locked
```

Testes do modelo de projetos e da interface:

```text
cargo build --locked
npm ci
npx playwright install chromium
python tests/run-ui.py
```

O modo `ATELIER_HEADLESS=1` permite testar a interface e o backend no Linux. A bandeja e os balões de notificação são implementados com as APIs Win32 e funcionam no Windows.

O workflow testa migração da agenda anterior, projetos, subtarefas, prioridades, prazos, Kanban, roadmap, cronômetro, calendário, central de avisos, adiamento, preferências de janela, plano e atualização. O núcleo testa lembretes antecipados, vencimento, recuperação, persistência/deduplicação, desativação, adiamento e checksums. A validação no Windows instala o programa, confirma o carregamento da interface, minimiza, restaura uma instância existente e fecha salvando os dados.

Para publicar uma nova versão, aumente `Cargo.toml`, atualize `Cargo.lock` e os metadados de versão do pacote de testes, valide e faça commit no `main`. A indicação de versão da interface vem do binário. O workflow publica `vX.Y.Z`, instalador, executável e checksum. Releases existentes não são sobrescritas. Alterações apenas em `mentor/` ou documentação não recompilam o programa.

O instalador não tem assinatura digital do autor; o bootstrapper WebView2 é obtido da Microsoft e sua assinatura é verificada pelo workflow.

### Plano dos trolls — 25 a 31/10/2026

Em 0.3.1 ou posterior: **Ajustes → Buscar plano do mentor → Aplicar atualização**.
O plano usa o início do dia que você configurou (campo obrigatório da agenda), sem impor horário de trabalho ou alarme: os prazos desta semana são datas nas descrições. A carga é revisável: domingo 3h, segunda a sexta 6h/dia, sábado 2h (35h). Entrega principal: prancha publicável dos trolls masculino e feminino do MMORPG autoral, com acabamento a lápis permitido como final; primeira prancha para revisão na quarta e finalização na sexta.

A revisão preserva tarefas e projetos anteriores, tempo, notas, conclusão e marcações de subtarefas com o mesmo ID. Se já aplicou o plano genérico antigo, arquive seus projetos antigos para não somar duas jornadas; eles não são apagados nem marcados como concluídos. Esta atualização não publica seu site e não acessa seus dados locais. Classes, demais props, símbolo, animais, arquitetura e escrita ficam para futuras semanas.
