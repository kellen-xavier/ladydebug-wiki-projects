---
title: "ADR 0001 - Versionamento automatizado na pipeline"
version: "1.0.0"
status: "ativo"
owner: "time"
updated: "2026-06-16"
systems: []
audience:
  - dev
  - qa
tags:
  - adr
  - pipeline
  - versionamento
---

# ADR 0001 - Versionamento automatizado na pipeline

> Template de ADR. Adapte comandos, nomes de branch e arquivos de manifesto ao
> seu projeto.

## Status

Aceito.

## Contexto

O repositorio segue Semantic Versioning e Conventional Commits como padroes de
governanca.

## Decisao

A pipeline passa a executar validacao em PRs e pushes para `develop` e `main`.

O versionamento automatico roda apenas em `main`, apos a validacao, criando:

- incremento SemVer baseado na ultima mensagem de commit;
- atualizacao do arquivo de manifesto do projeto (ex.: `package.json`,
  `pyproject.toml`, `Cargo.toml`) e seu lockfile;
- commit `chore(release): X.Y.Z [skip ci]`;
- tag Git anotada `vX.Y.Z`.

O incremento padrao e:

| Padrao de commit | Incremento |
| --- | --- |
| `BREAKING CHANGE:` ou `tipo!:` | MAJOR |
| `feat:` | MINOR |
| demais commits | PATCH |

Tambem e possivel sobrescrever o incremento com a variavel `VERSION_BUMP`
usando `major`, `minor` ou `patch`.

## Consequencias

- PRs validam qualidade antes do merge.
- Releases ficam rastreaveis por tag Git.
- Commits de versionamento usam `[skip ci]` para evitar loop de pipeline.
- A estrategia depende de mensagens aderentes a Conventional Commits.
- O passo de manifesto e agnostico de ecossistema: troque o arquivo alvo pelo
  do seu stack sem alterar o resto do fluxo.
