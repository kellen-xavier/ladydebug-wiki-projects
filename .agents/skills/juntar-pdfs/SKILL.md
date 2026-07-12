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

## Script: `juntar_pdfs.rb`

Entregue este script Ruby ao usuário. Ele é autocontido e não depende de gems externas.

```ruby
#!/usr/bin/env ruby
# frozen_string_literal: true

require 'optparse'
require 'fileutils'
require 'tmpdir'
require 'open3'

GS_QUALITY = 'printer' # printer | ebook | screen

def log(msg)   = puts("[INFO]  #{msg}")
def ok(msg)    = puts("[  OK] #{msg}")
def warn(msg)  = $stderr.puts("[WARN]  #{msg}")
def error(msg) = $stderr.puts("[ERROR] #{msg}")

def natural_sort(arr)
  arr.sort_by { |s| s.scan(/(\d+)|(\D+)/).map { |n, a| n ? n.to_i : a.downcase } }
end

def qpdf_available?    = system('which qpdf > /dev/null 2>&1')
def ghostscript_available? = system('which gs > /dev/null 2>&1')

def merge_pdfs(pdf_list, output_path)
  return false if pdf_list.empty?
  if pdf_list.size == 1
    FileUtils.cp(pdf_list.first, output_path)
    return true
  end
  cmd = ['qpdf', '--empty', '--pages'] + pdf_list + ['--', output_path]
  _, stderr, status = Open3.capture3(*cmd)
  unless status.success?
    error "  qpdf falhou: #{stderr.strip}"
    return false
  end
  true
end

def compress_pdf(input_path, output_path)
  unless ghostscript_available?
    FileUtils.cp(input_path, output_path)
    return
  end
  cmd = [
    'gs', '-sDEVICE=pdfwrite', '-dCompatibilityLevel=1.5',
    "-dPDFSETTINGS=/#{GS_QUALITY}", '-dNOPAUSE', '-dQUIET', '-dBATCH',
    '-dColorImageResolution=150', '-dGrayImageResolution=150',
    '-dMonoImageResolution=300', "-sOutputFile=#{output_path}", input_path
  ]
  _, stderr, status = Open3.capture3(*cmd)
  if status.success? && File.exist?(output_path)
    original_kb   = (File.size(input_path)  / 1024.0).round(1)
    compressed_kb = (File.size(output_path) / 1024.0).round(1)
    savings = ((1 - compressed_kb.to_f / original_kb) * 100).round(1)
    log "  Compressão: #{original_kb} KB → #{compressed_kb} KB (#{savings}% menor)"
  else
    warn "  GhostScript falhou, usando PDF sem compressão"
    FileUtils.cp(input_path, output_path)
  end
end

def process_id_folder(id_folder, output_dir)
  id_name = File.basename(id_folder)
  log "Processando: #{id_name}"
  pdfs = natural_sort(
    Dir.glob(File.join(id_folder, '*.pdf')) +
    Dir.glob(File.join(id_folder, '*.PDF'))
  )
  if pdfs.empty?
    warn "  Nenhum PDF encontrado em: #{id_name}"
    return nil
  end
  log "  #{pdfs.size} PDF(s) em ordem:"
  pdfs.each { |f| log "    • #{File.basename(f)}" }
  Dir.mktmpdir("juntar_#{id_name}_") do |tmp|
    merged_tmp = File.join(tmp, "#{id_name}_merged.pdf")
    unless merge_pdfs(pdfs, merged_tmp)
      error "  Falha ao juntar PDFs de: #{id_name}"
      return nil
    end
    output_pdf = File.join(output_dir, "#{id_name}.pdf")
    compress_pdf(merged_tmp, output_pdf)
    ok "Salvo: #{output_pdf}"
    output_pdf
  end
end

options = { output: nil }
parser = OptionParser.new do |opts|
  opts.banner = "Uso: ruby #{File.basename($PROGRAM_NAME)} [opções] <caminho/Evidências>"
  opts.on('-o', '--output DIR', 'Pasta de saída (padrão: merged/ dentro de Evidências)') { |d| options[:output] = d }
  opts.on('-h', '--help', 'Exibe esta ajuda') { puts opts; exit }
end
parser.parse!

if ARGV.empty?
  error 'Informe o caminho da pasta Evidências.'
  puts parser; exit 1
end

evidencias_path = File.expand_path(ARGV[0])
unless Dir.exist?(evidencias_path)
  error "Pasta não encontrada: #{evidencias_path}"; exit 1
end
unless qpdf_available?
  error 'qpdf não encontrado. Instale com: sudo apt install qpdf'; exit 1
end

output_dir = options[:output] || File.join(evidencias_path, 'merged')
FileUtils.mkdir_p(output_dir)

id_folders = natural_sort(
  Dir.glob(File.join(evidencias_path, '*/'))
     .select { |f| File.directory?(f) }
     .reject { |f| File.expand_path(f) == File.expand_path(output_dir) }
)

if id_folders.empty?
  error "Nenhuma subpasta encontrada em: #{evidencias_path}"; exit 1
end

puts '=' * 60
puts "Pasta Evidências : #{evidencias_path}"
puts "Saída            : #{output_dir}"
puts "Subpastas        : #{id_folders.size}"
puts '=' * 60
puts

results = { ok: [], fail: [] }
id_folders.each do |folder|
  pdf = process_id_folder(folder, output_dir)
  pdf ? results[:ok] << pdf : results[:fail] << folder
  puts
end

puts '=' * 60
puts "Juntados com sucesso : #{results[:ok].size}"
puts "Falhas               : #{results[:fail].size}"
unless results[:fail].empty?
  puts "\nPastas com falha:"
  results[:fail].each { |f| puts "  • #{f}" }
end
exit(results[:fail].empty? ? 0 : 1)
```

## Uso

```bash
# Uso básico — salva em Evidências/merged/
ruby juntar_pdfs.rb "caminho/Evidências"

# Com pasta de saída customizada
ruby juntar_pdfs.rb "caminho/Evidências" --output "caminho/saida"

# Ajuda
ruby juntar_pdfs.rb --help
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
- Não converte `.docx` — para isso use o script `docx_to_pdf.rb` separadamente
- PDFs protegidos por senha causam falha no `qpdf` para aquela pasta
