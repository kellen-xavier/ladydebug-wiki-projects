# Registro de Alterações

O que mudou neste repositório a cada versão, da mais recente para a mais antiga.

Este repositório é uma base "template" de documentação para projetos com IA
(veja o [README](/README.md)), então cada entrada descreve o efeito prático da
mudança para quem copia esta base — não o detalhe técnico do commit.

As entradas são escritas a partir das mensagens de commit, que seguem
[Conventional Commits](https://www.conventionalcommits.org/pt-br/), e as versões
seguem [Semantic Versioning](https://semver.org/lang/pt-BR/). Enquanto o
versionamento automatizado na pipeline não estiver ativo, este arquivo é
atualizado à mão a cada versão — daí a importância de escrever bem a mensagem
de commit.

## Como ler

| Seção | O que significa |
| --- | --- |
| Mudanças importantes | Alterou a forma de trabalhar; leia antes de seguir um procedimento antigo |
| Novidades | Conteúdo, skill ou script novo no repositório |
| Correções | Algo que estava errado e foi corrigido |
| Documentação | Documento revisado, ampliado ou reorganizado |
| Manutenção interna | Ajuste técnico sem efeito no conteúdo (estrutura de pastas, build, formatação) |
| Outras alterações | Alteração cuja mensagem não seguiu o padrão de commit |

> **Não publicado** reúne o que já foi aprovado mas ainda não entrou em uma versão.

## Como a mensagem de commit vira uma entrada

| Prefixo do commit | Seção onde aparece |
| --- | --- |
| `feat!:` ou `BREAKING CHANGE:` no corpo | Mudanças importantes |
| `feat:` | Novidades |
| `fix:` | Correções |
| `docs:` | Documentação |
| `chore:`, `refactor:` (ou `refact:`), `build:`, `ci:`, `test:` | Manutenção interna |
| sem prefixo reconhecido | Outras alterações |

E o incremento de versão:

| Padrão de commit | Incremento |
| --- | --- |
| `BREAKING CHANGE:` ou `tipo!:` | MAJOR |
| `feat:` | MINOR |
| demais commits | PATCH |

---

## Não publicado

### Documentação

- Adicionado este registro de alterações, com o histórico do repositório até a
  versão 0.1.0 e a convenção de escrita das próximas entradas.

---

## 0.1.0 — 2026-07-16

Estado inicial do repositório, anterior à adoção do registro de alterações.
Reúne a base de estudo da wiki, as skills de agente e os scripts de conversão
de documentos.

### Novidades

- Base inicial da wiki de estudos com IA, incluindo o esqueleto de navegação do
  MkDocs (`mkdocs.yml`) e a arquitetura de referência da documentação.
- Referências de estudo no README (Codex, regras de desenvolvimento e memória de
  longo prazo para projetos).
- Regras de agente do repositório em `AGENTS.md` e `.agents/AGENTS.md`, com o
  fluxo obrigatório de revisão documental e os critérios mínimos de aceite.
- Skill `juntar-pdfs` e o script `scripts/juntar_pdfs.rb`, para mesclar arquivos
  PDF em sequência.
- Skill `docx-to-pdf` e skill `wiki-writer`.
- Skill `md2-to-docx` e o conversor de Markdown para DOCX em Rust
  (`scripts/md2-to-docx.rs`), preservando conteúdo e indentação de forma
  verbatim.
- Conversor de DOCX para PDF reescrito em Rust (`scripts/docx_to_pdf.rs`),
  substituindo a versão anterior da automação.

### Correções

- Base da skill de conversão `.docx` para `.pdf` e da mesclagem sequencial de
  PDFs, que não funcionavam como descrito.
- Registro do binário `docx_to_pdf` no `Cargo.toml` e correção da detecção de
  sucesso da conversão, que reportava êxito mesmo quando o arquivo não era
  gerado.

### Manutenção interna

- Organização inicial do repositório.
- Reestruturação das pastas do projeto, separando `.agents/skills/`, `scripts/`
  e `src/`.
