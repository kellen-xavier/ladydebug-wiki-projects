---
title: "ADR 0002 - Diagramas da arquitetura proposta"
version: "1.0.0"
status: "proposto"
owner: "time"
updated: "2026-07-29"
audience:
  - analista
  - dev
  - qa
tags:
  - adr
  - organizacao
  - dominio
  - diagrama
---

# ADR 0002 - Diagramas da arquitetura proposta

> Template de ADR. Complemento visual do
> [ADR 0002 - organizacao](0002-organizacao-orientada-a-dominio.md). Os nomes de
> sistema e as contagens sao exemplos ilustrativos; troque pelos do seu projeto.

Os diagramas 1 e 2 registram o estado **anterior** a migracao, para consulta
historica. Os diagramas 3 a 6 refletem o estado alvo e o que falta.

## 1. Estado atual: tres criterios competindo

O problema nao e "pasta solta". E que tres criterios de organizacao diferentes
ocupam o mesmo nivel da arvore, entao nao ha como prever onde procurar.

```mermaid
graph TD
    RAIZ["Raiz do repositorio"]

    RAIZ --> S1["sistema-a/"]
    RAIZ --> S2["sistema-b/"]
    RAIZ --> S3["sistema-c/"]
    RAIZ --> T1["database/"]
    RAIZ --> T2["integracoes/"]
    RAIZ --> T3["fluxos/"]
    RAIZ --> D1["dominio/"]
    D1 --> D2["sistema-a/"]

    classDef sistema fill:#7c3aed,stroke:#5b21b6,color:#fff
    classDef artefato fill:#0f766e,stroke:#134e4a,color:#fff
    classDef dominio fill:#b45309,stroke:#78350f,color:#fff

    class S1,S2,S3 sistema
    class T1,T2,T3 artefato
    class D1,D2 dominio
```

Legenda: roxo = organizado **por sistema** · verde = **por tipo de artefato** ·
laranja = **por dominio**.

`sistema-c/` e `sistema-b/` parecem soltas porque seguem o criterio roxo enquanto
suas vizinhas seguem o verde. O criterio laranja existe mas contem so um sistema.

## 2. A duplicacao mais grave

`sistema-a` existe em dois lugares, com as mesmas subpastas e divisao arbitraria
de conteudo.

```mermaid
graph LR
    subgraph RAIZ["sistema-a/ (raiz)"]
        A1["cadastro/ — 2"]
        A2["area-x/ — 28"]
        A3["area-y/ — 3"]
    end

    subgraph DOM["dominio/sistema-a/"]
        B1["cadastro/ — 2"]
        B2["area-x/ — 3"]
        B3["area-y/ — 11"]
        B4["modulo-1/ — 61"]
        B5["modulo-2/ — 24"]
        B6["fluxos/ — 6"]
    end

    A1 -.->|"mesma pasta"| B1
    A2 -.->|"mesma pasta"| B2
    A3 -.->|"mesma pasta"| B3
```

Nao e "antigo vs novo": `area-x` tem 28 na raiz e 3 no dominio, `area-y` tem o
inverso. Nenhum leitor sabe qual e o canonico.

## 3. Estrutura proposta

Duas perguntas cobrem o repositorio inteiro: **de qual sistema?** e
**narrativa ou referencia?**

```mermaid
graph TD
    RAIZ["Raiz"]

    RAIZ --> PORTA["README · GLOSSARIO · SISTEMAS<br/>porta de entrada"]
    RAIZ --> DOM["dominio/<br/>NARRATIVA"]
    RAIZ --> REF["database/ · integracoes/<br/>REFERENCIA"]
    RAIZ --> GOV["adr/ · scripts/<br/>governanca"]

    DOM --> H1["sistema-a/"]
    DOM --> H3["sistema-b/"]
    DOM --> H4["sistema-c/"]

    H1 --> HF["fluxos/ · cadastro/<br/>modulo-1/ · modulo-2/ · area-x/ · area-y/"]
    H3 --> RF["index.md · fluxos/"]
    H4 --> SF["index.md · fluxos/ · img/"]

    REF --> R1["database/tabelas/<br/>dicionario de dados"]
    REF --> R2["database/queries/<br/>por subsistema"]
    REF --> R3["integracoes/<br/>contratos entre sistemas"]

    classDef narrativa fill:#b45309,stroke:#78350f,color:#fff
    classDef referencia fill:#0f766e,stroke:#134e4a,color:#fff
    classDef neutro fill:#374151,stroke:#1f2937,color:#fff

    class DOM,H1,H3,H4,HF,RF,SF narrativa
    class REF,R1,R2,R3 referencia
    class PORTA,GOV neutro
```

`dominio/` espelha os sistemas declarados em `SISTEMAS.md`. Referencia fica
compartilhada porque varios fluxos apontam para a mesma tabela, e porque
integracao e por definicao *entre* sistemas.

## 4. Mapa de movimentacao

De onde cada coisa sai e para onde vai.

```mermaid
graph LR
    O1["sistema-b/*.sql<br/>19 arquivos"] --> N1["database/queries/sistema-b/"]
    O2["sistema-b/doc-tabela-exemplo.md"] --> N2["database/tabelas/"]
    O3["sistema-b/narrativa-a.md<br/>narrativa-b.md"] --> N3["dominio/sistema-b/"]
    O4["fluxos/sistema-b/"] --> N4["dominio/sistema-b/fluxos/"]
    O5["sistema-c/"] --> N5["dominio/sistema-c/"]
    O9["sistema-b/payload-integracao.txt"] --> N9["integracoes/<br/>mensagem-exemplo-integracao.txt"]
    O6["sistema-a/<br/>na raiz"] --> N6["dominio/sistema-a/"]
    O7["fluxos/*.md<br/>5 soltos, todos do sistema-a"] --> N7["dominio/sistema-a/fluxos/"]
    O8["fluxos/<br/>vazia ao fim"] --> N8["removida"]

    classDef antes fill:#991b1b,stroke:#7f1d1d,color:#fff
    classDef depois fill:#0f766e,stroke:#134e4a,color:#fff

    class O1,O2,O3,O4,O5,O6,O7,O8,O9 antes
    class N1,N2,N3,N4,N5,N6,N7,N8,N9 depois
```

## 5. Onde eu coloco meu arquivo?

Arvore de decisao para quem vai contribuir — inclusive quem nao e tecnico.

```mermaid
graph TD
    START["Tenho um arquivo novo"] --> Q1{"Ele responde<br/>'como funciona o processo X'?"}

    Q1 -->|"Sim: e narrativa"| Q2{"De qual sistema<br/>ele fala?"}
    Q1 -->|"Nao: e consulta pontual"| Q3{"O que ele descreve?"}

    Q2 --> A1["dominio/sistema-a/fluxos/"]
    Q2 --> A2["dominio/sistema-b/fluxos/"]
    Q2 --> A3["dominio/sistema-c/fluxos/"]

    Q3 --> B1["Uma tabela<br/>database/tabelas/"]
    Q3 --> B2["Uma query SQL<br/>database/queries/{sistema}/"]
    Q3 --> B3["Um contrato entre sistemas<br/>integracoes/"]

    Q2 -.->|"cruza sistemas?"| NOTA["Vai no sistema que<br/>ORQUESTRA o fluxo,<br/>com link para os outros"]

    classDef pergunta fill:#374151,stroke:#1f2937,color:#fff
    classDef narrativa fill:#b45309,stroke:#78350f,color:#fff
    classDef referencia fill:#0f766e,stroke:#134e4a,color:#fff

    class START,Q1,Q2,Q3 pergunta
    class A1,A2,A3,NOTA narrativa
    class B1,B2,B3 referencia
```

## 6. Ordem de execucao das fases

Cada fase e uma PR independente. A dependencia importa: consolidar a duplicacao
vem antes de tudo, senao as fases seguintes movem conteudo ambiguo.

```mermaid
graph LR
    F1["Fase 1<br/>Consolidar sistema-a<br/>duplicado"] --> F2["Fase 2<br/>sistema-c para<br/>dominio/sistema-c"]
    F1 --> F3["Fase 3<br/>Dividir<br/>sistema-b"]
    F2 --> F4["Fase 4<br/>Mover fluxos<br/>remove fluxos/"]
    F3 --> F4
    F4 --> F5["Fase 5<br/>index.md e .order<br/>navegacao da wiki"]
    F5 --> F6["Fase 6<br/>Varredura de<br/>links quebrados"]

    classDef critica fill:#991b1b,stroke:#7f1d1d,color:#fff
    classDef normal fill:#0f766e,stroke:#134e4a,color:#fff

    class F1 critica
    class F2,F3,F4,F5,F6 normal
```

Fase 1 em vermelho por ser bloqueante. Fases 2 e 3 podem correr em paralelo.
