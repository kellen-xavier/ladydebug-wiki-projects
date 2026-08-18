---
name: juntar-pdfs
description: >
  Junta todos os PDFs em um único PDF compilado, preservando
  ordem natural dos arquivos e comprimindo o resultado sem perda de qualidade de imagem.
  Use esta skill sempre que o usuário quiser mesclar, compilar, juntar ou consolidar PDFs
  organizados de qualquer hierarquia onde cada pasta contém um ou mais PDFs que devem virar um arquivo só.
  Também se aplica quando o usuário mencionar "juntar PDFs por pasta",
  "consolidar arquivos PDF", "unir PDFs em sequência" ou estruturas similares.
---

# Juntar PDFs por Pasta

## Visão Geral

Compilar os PDFs quando solicitado para mesclar em um único arquivo PDF.

1. Lista todos os PDFs em **ordem natural** (`1, 2, 10` — nunca `1, 10, 2`)
2. **Junta em sequência** sem embaralhar páginas
3. **Comprime** o PDF final via GhostScript preservando qualidade de imagem
4. Salva como `PDF merge consolidado.pdf` na pasta de saída

Resultado gerado em `\Pasta/merged/` (ou pasta customizada via `--output`):

```txt
merged/
  ID_0001.pdf   ← [DET] ID 0001 (ordem natural)
  ID_0002.pdf
  ID_0010.pdf   ← [DET] ID 00010
```

## Script: `script/juntar_pdfs.rb`

Script Ruby autocontido, sem gems externas — só a stdlib
(`optparse`, `fileutils`, `tmpdir`, `open3`). Ver
[`script/juntar_pdfs.rb`](script/juntar_pdfs.rb).

## Uso

```bash
# Uso básico — salva em Evidências/merged/
ruby script/juntar_pdfs.rb "caminho/Evidências"

# Com pasta de saída customizada
ruby script/juntar_pdfs.rb "caminho/Evidências" --output "caminho/saida"

# Ajuda
ruby script/juntar_pdfs.rb --help
```

## Pré-requisitos

```bash
sudo apt install qpdf ghostscript   # Debian / Ubuntu
```

| Ferramenta    | Função                                      |
|---------------|---------------------------------------------|
| `qpdf`        | Mescla PDFs em sequência sem recodificar    |
| `ghostscript` | Comprime o PDF final preservando imagens    |
| Ruby stdlib   | Nenhuma gem externa necessária              |

## Configuração de Compressão

Altere `GS_QUALITY` no topo do script conforme a necessidade:

| Valor       | Qualidade       | Indicado para                          |
|-------------|-----------------|----------------------------------------|
| `printer`   | Alta (padrão)   | Evidências com imagens e capturas      |
| `ebook`     | Média           | Documentos majoritariamente textuais   |
| `screen`    | Baixa           | Visualização web, tamanho mínimo       |

## Comportamentos Importantes

- **Ordem natural garantida**: `pdf_1`, `pdf_2`, `pdf_10` — nunca `1, 10, 2`
- **Sem embaralhamento**: páginas são concatenadas na ordem exata dos arquivos
- **Pasta de saída excluída**: a pasta `merged/` nunca é processada como subpasta de ID
- **Tolerante a falhas**: uma pasta com erro não interrompe o processamento das demais
- **Saída de progresso**: lista cada PDF incluído e exibe relatório de compressão

## Limitações

- Processa apenas **um nível** de subpastas
- Não converte `.docx` — para isso use a skill `docx-to-pdf` separadamente
- PDFs protegidos por senha causam falha no `qpdf` para aquela pasta
