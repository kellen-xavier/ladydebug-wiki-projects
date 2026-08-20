---
name: pdf-spec-para-imagem
description: >
  Extrai o conteudo de um PDF de especificacoes detalhadas de forma determinista
  (script, nao leitura do documento) e gera imagem usando somente os detalhes
  extraidos: uma ficha visual desenhada por codigo e/ou uma ilustracao produzida
  pelo agente de imagem que o usuario configurou na propria maquina. Use esta
  skill sempre que o pedido for "gerar imagem a partir de um PDF de
  especificacao", "extrair os detalhes do PDF e desenhar o produto", "ficha
  visual da especificacao", "ilustracao do item especificado no PDF" ou
  "transformar spec em imagem". Nao interpreta, nao resume e nao completa
  lacunas: o que nao foi extraido nao entra na imagem.
---

# PDF de especificacao para imagem

## Visao geral

Converter um PDF de especificacoes em imagem **sem que o agente leia o PDF**.
Quem le o arquivo e um script; o que chega na imagem e somente aquilo que o
script conseguiu extrair, com pagina e linha de origem registradas.

```txt
PDF                  extracao deterministica        briefing validado          imagem
especificacao.pdf -> knowledge/extracted/<slug>/ -> prompt.txt + briefing.md -> ficha.svg
   (preservado em      spec.json / texto.txt         (so valores extraidos)     ilustracao.png
    knowledge/raw/)     spec.md
```

Duas saidas, do mesmo briefing:

| Saida | Como e produzida | Fidelidade |
| --- | --- | --- |
| `ficha.svg` (+ `.png`) | desenhada por codigo a partir do `spec.json` | 1:1 com o texto extraido |
| `ilustracao.png` | gerada pelo **agente de imagem configurado pelo usuario** | ilustrativa, guiada pelo briefing |

## Regra de ouro

**O agente nao le o PDF.** Nao abrir o arquivo com ferramenta de leitura, nao
olhar as paginas, nao descrever o documento de memoria. A unica porta de entrada
do conteudo e `scripts/extrair_spec_pdf.py`.

Consequencias praticas:

- nenhum atributo pode entrar na imagem sem constar em `texto.txt`;
- o que faltar fica marcado como `pendente-validacao` — nunca preenchido "por
  coerencia";
- se o PDF nao tiver camada de texto, a extracao **falha de proposito**
  (codigo 2) e pede OCR. Ler as paginas visualmente para contornar isso quebra
  a skill.

O portao nao e so uma recomendacao: `gerar_imagem_spec.py` compara cada linha do
prompt com o texto extraido e aborta com codigo 3 se aparecer qualquer valor sem
origem no PDF.

## Fluxo

1. **Preservar o original** em `knowledge/raw/` (o proprio extrator faz com
   `--preservar`; nunca sobrescreve arquivo existente).
2. **Extrair** para `knowledge/extracted/<slug>/`.
3. **Conferir** a extracao (`spec.md`): campos vazios, tabelas quebradas,
   secoes ausentes viram pendencias declaradas, nao chutes.
4. **Gerar o briefing e a(s) imagem(ns)** em `knowledge/curated/<slug>/imagem/`.
5. **Reportar** arquivos gerados, fonte, pendencias e validacoes executadas.

## Scripts

| Script | Papel |
| --- | --- |
| [`scripts/extrair_spec_pdf.py`](../../../scripts/extrair_spec_pdf.py) | le o PDF e estrutura o conteudo com rastreabilidade |
| [`scripts/gerar_imagem_spec.py`](../../../scripts/gerar_imagem_spec.py) | monta o briefing, valida a origem de cada valor e gera as imagens |

Ambos usam **somente a biblioteca padrao do Python 3.8+**. O extrator tem motor
proprio de PDF (Flate/LZW/ASCII85, object streams, xref streams, fontes simples
e Type0/Identity-H com ToUnicode) e usa `pdftotext -layout` automaticamente
quando ele existir na maquina.

## Uso

### 1. Extrair

```bash
# extrai e preserva o original em knowledge/raw/
python3 scripts/extrair_spec_pdf.py especificacao.pdf --preservar

# pasta de saida e nome proprios; forca o motor interno
python3 scripts/extrair_spec_pdf.py spec.pdf -o knowledge/extracted -n luminaria-orion -m interno
```

| Opcao | Funcao |
| --- | --- |
| `-o, --saida` | diretorio base de saida (padrao: `knowledge/extracted`) |
| `-n, --nome` | nome da pasta (padrao: slug do arquivo) |
| `-m, --motor` | `auto` (padrao), `interno` ou `pdftotext` |
| `--preservar [DIR]` | copia o PDF para `knowledge/raw/` (ou `DIR`) sem sobrescrever |
| `--min-chars` | minimo de caracteres para aceitar que ha camada de texto (padrao: 40) |
| `--forcar` | sobrescreve a pasta de saida |

Saidas: `texto.txt` (texto cru por pagina), `spec.json` (campos, titulos, listas,
atributos visuais — cada um com pagina e linha) e `spec.md` (leitura humana).

### 2. Gerar a imagem

```bash
# ficha visual + ilustracao pelo agente configurado
python3 scripts/gerar_imagem_spec.py knowledge/extracted/luminaria-orion

# so a ficha deterministica (sem IA, sem rede)
python3 scripts/gerar_imagem_spec.py knowledge/extracted/luminaria-orion --tipo ficha

# so a ilustracao, escolhendo o agente
python3 scripts/gerar_imagem_spec.py knowledge/extracted/luminaria-orion --tipo ilustracao --agent comfy-local

# valida o briefing sem chamar o agente
python3 scripts/gerar_imagem_spec.py knowledge/extracted/luminaria-orion --dry-run

# quais agentes estao configurados nesta maquina
python3 scripts/gerar_imagem_spec.py --listar-agents
```

| Opcao | Funcao |
| --- | --- |
| `-t, --tipo` | `ficha`, `ilustracao` ou `ambos` (padrao) |
| `-o, --saida` | diretorio base (padrao: `knowledge/curated`) |
| `-a, --agent` | agente de imagem (padrao: `agent_padrao` da configuracao) |
| `-c, --config` | arquivo de configuracao dos agentes |
| `--largura`, `--altura` | dimensoes pedidas ao agente (padrao: 1024x1024) |
| `--max-campos` | teto de campos levados ao briefing (padrao: 40) |
| `--dry-run` | monta e valida tudo, sem chamar o agente |

## O agente de imagem e do usuario

A skill nao embute provedor nem chave: ela executa **o gerador que a pessoa ja
usa**. Copie o exemplo e ajuste:

```bash
cp .agents/config/imagem-agents.exemplo.json .agents/config/imagem-agents.json
```

```json
{
  "agent_padrao": "cli-local",
  "agents": {
    "cli-local": {
      "descricao": "meu gerador de imagem",
      "comando": ["meu-gerador", "--prompt-file", "{prompt_arquivo}", "--out", "{saida}"],
      "prompt_via": "arquivo",
      "saida_via": "arquivo",
      "timeout_s": 300
    }
  }
}
```

- Placeholders: `{prompt}`, `{prompt_arquivo}`, `{saida}`, `{largura}`, `{altura}`.
- `prompt_via`: `arquivo` | `argumento` | `stdin`. `saida_via`: `arquivo` | `stdout`.
- Ordem de busca da configuracao: `--config` -> `$SPEC_IMAGEM_CONFIG` ->
  `.agents/config/imagem-agents.json` -> `~/.config/spec-imagem/agents.json`.
- O comando roda **sem shell**, com o ambiente do seu terminal. Exporte a chave
  de API no shell (`export OPENAI_API_KEY=...`); **nao** escreva chave no JSON.
- `.agents/config/imagem-agents.json` esta no `.gitignore`: e configuracao de
  maquina, nao do repositorio.

## Saidas e rastreabilidade

```txt
knowledge/
  raw/especificacao.pdf                     original preservado
  extracted/<slug>/spec.json                estrutura + pagina/linha de cada item
  extracted/<slug>/texto.txt                texto cru (base da validacao)
  extracted/<slug>/spec.md                  leitura humana da extracao
  curated/<slug>/imagem/prompt.txt          prompt exato enviado ao agente
  curated/<slug>/imagem/briefing.md         cada item do prompt com pagina e linha
  curated/<slug>/imagem/ficha.svg (.png)    ficha visual deterministica
  curated/<slug>/imagem/ilustracao.png      imagem do agente configurado
  curated/<slug>/imagem/manifesto.json      SHA-256 do PDF, do spec, comando e agente usados
```

## Codigos de saida

| Codigo | Significado | O que fazer |
| --- | --- | --- |
| `0` | tudo certo | seguir |
| `1` | erro de uso (arquivo ausente, pasta ocupada, agente inexistente) | corrigir o comando |
| `2` | PDF sem camada de texto | rodar OCR (`ocrmypdf`) e extrair de novo — **nao** ler as paginas |
| `3` | briefing com valor sem origem no PDF | remover o valor inventado; nao contornar a validacao |

## Pre-requisitos

| Componente | Situacao |
| --- | --- |
| Python 3.8+ | obrigatorio (so biblioteca padrao) |
| `poppler-utils` (`pdftotext`) | opcional — melhora o layout de tabelas |
| `rsvg-convert`, `inkscape`, `magick` ou `chromium` | opcional — converte a ficha para PNG |
| `ocrmypdf` / `tesseract` | necessario apenas para PDF digitalizado |
| agente de imagem do usuario | necessario apenas para `--tipo ilustracao` |

```bash
sudo apt install poppler-utils librsvg2-bin ocrmypdf   # Debian / Ubuntu
brew install poppler librsvg ocrmypdf                  # macOS
```

## Comportamentos importantes

- **Origem intacta**: o PDF nunca e alterado; `--preservar` copia, nao move.
- **Rastreabilidade item a item**: todo campo carrega pagina e linha.
- **Validacao antes da geracao**: prompt com valor sem lastro aborta (codigo 3).
- **Determinismo na ficha**: mesma extracao, mesmo SVG — sem IA, sem rede.
- **Ficha sempre disponivel**: se nenhum conversor SVG->PNG existir, entrega SVG.
- **Sem shell na chamada do agente**: argumentos passam como lista, com timeout.
- **Manifesto assinado por hash**: `manifesto.json` guarda SHA-256 do PDF, do
  `spec.json` e da imagem gerada, alem do comando executado.

## Limitacoes

- PDF digitalizado exige OCR previo (a skill recusa em vez de adivinhar).
- Tabelas de layout muito complexo podem sair como pares chave/valor imperfeitos;
  conferir `spec.md` antes de gerar a imagem.
- Fonte embutida sem `ToUnicode` e com codificacao propria pode render `�`;
  o aviso aparece na extracao e o trecho vira pendencia.
- A fidelidade da `ilustracao.png` depende do agente configurado — a garantia da
  skill e sobre o **briefing**, nao sobre o desenho do modelo.
- A ficha cobre campos, atributos visuais e observacoes; nao reproduz imagens,
  graficos ou desenhos tecnicos embutidos no PDF.

## Ao concluir, informe

- arquivos gerados (extracao, briefing e imagens);
- fonte (PDF, SHA-256, paginas) e motor de extracao usado;
- agente de imagem acionado, quando houver;
- pendencias (`pendente-validacao`) e campos nao reconhecidos;
- validacoes executadas (portao do briefing, codigo de saida dos scripts).
