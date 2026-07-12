# md2docx

Converte um arquivo Markdown (`.md`) em `.docx` **preservando 100% do conteúdo e da indentação** do arquivo original.

## Comportamento

Conversão **verbatim**: cada linha do `.md` vira um parágrafo em fonte monoespaçada (Consolas), com todos os espaços preservados via `xml:space="preserve"`. A sintaxe Markdown **não é interpretada** — `#`, `-`, cercas de código (```` ``` ````), indentação de listas e blocos permanecem exatamente como no fonte. É a única abordagem que garante zero alteração de conteúdo/indentação.

Preservado: espaços de indentação (2/4/8...), espaços à direita, acentos/Unicode, e `<` `>` `&` (escapados corretamente para o XML, exibidos como o caractere original no Word).

## Build (uma vez)

Sem dependências pesadas — só o crate `zip`. Um `.docx` é apenas um ZIP com alguns XMLs, gerados diretamente.

```bash
cargo build --release
# binário em: target/release/md2docx  (~415 KB)
```

## Uso

```bash
md2docx <entrada.md> [saida.docx]
```

- Saída omitida → mesmo caminho da entrada com extensão `.docx`.

```bash
md2docx SKILL.md              # -> SKILL.docx
md2docx notas.md out/nota.docx
```

## Notas

- **Finais de linha**: CRLF/CR são normalizados para quebra de parágrafo (o `\r` é controle de fim de linha, não indentação). Espaços e tabs de indentação ficam intactos.
- **Tabs**: preservados como caractere literal. Indentação por espaços (padrão em Markdown) alinha perfeitamente na fonte monoespaçada; arquivos com indentação por tab podem alinhar conforme os tab stops do Word.
- **BOM UTF-8**: removido se presente (não é conteúdo visível).

## Variante alternativa (sob demanda)

Este skill entrega **fidelidade textual** (o `.md` cru dentro do `.docx`). Se em vez disso você quiser um `.docx` **renderizado** (títulos como Heading, listas como bullets reais, negrito etc.), isso é uma conversão diferente — que por definição altera a forma como a indentação/sintaxe aparece. Peça e eu ajusto o programa para esse modo.
