# AGENTS.md

## Objetivo do repositório

Este repositório centraliza conhecimento técnico, funcional e operacional do projeto.

A fonte de verdade da wiki é Markdown versionado em `docs/`.

Arquivos em `knowledge/raw/` são fontes brutas e não devem ser tratados como documentação final, mas como exemplo

## Regras gerais

- Não criar documentação sem indicar fonte.
- Não promover hipótese para regra.
- Não sobrescrever página existente sem preservar contexto relevante.
- Preferir linguagem clara para páginas de negócio e operação.
- Separar conteúdo técnico de conteúdo funcional quando o público for diferente.
- Manter links relativos entre páginas.
- Toda decisão relevante deve ser registrada em `docs/decisoes/`.
- Toda regra de negócio deve informar sistema, processo, evidência e data de revisão.
- Toda integração deve indicar origem, destino, contrato, payload, status e falhas conhecidas.

## Estrutura obrigatória

- `docs/`: wiki publicada.
- `knowledge/raw/`: documentos originais.
- `knowledge/extracted/`: extrações automáticas.
- `knowledge/curated/`: material intermediário revisado.
- `.agents/skills/`: skills do agente.
- `.agents/templates/`: templates oficiais.
- `scripts/`: scripts úteis para o projeto.

## Critérios de saída

Ao finalizar qualquer alteração, informe:

- arquivos modificados;
- fonte utilizada;
- páginas criadas ou atualizadas;
- pendências;
- validações executadas.
