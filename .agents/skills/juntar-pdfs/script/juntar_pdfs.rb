#!/usr/bin/env ruby
# frozen_string_literal: true

# juntar_pdfs.rb
#
# Para cada subpasta (ID_NUMERO) dentro de Evidências:
#   1. Lista todos os PDFs em ordem natural (1, 2, 10 — não 1, 10, 2)
#   2. Junta todos em um único PDF compilado
#   3. Comprime o PDF final preservando qualidade de imagem
#
# Uso:
#   ruby juntar_pdfs.rb <caminho/para/Evidências>
#   ruby juntar_pdfs.rb <caminho/para/Evidências> --output <pasta_de_saida>
#   ruby juntar_pdfs.rb --help

require 'optparse'
require 'fileutils'
require 'tmpdir'
require 'open3'

# ─── Configurações ────────────────────────────────────────────────────────────

# Qualidade de compressão GhostScript:
# "printer" → boa qualidade, tamanho médio (recomendado para evidências com imagens)
# "ebook"   → qualidade razoável, menor tamanho
# "screen"  → menor qualidade, menor tamanho
GS_QUALITY = 'printer'

# ─── Helpers ──────────────────────────────────────────────────────────────────

def log(msg)   = puts("[INFO]  #{msg}")
def ok(msg)    = puts("[  OK] #{msg}")
def warn(msg)  = $stderr.puts("[WARN]  #{msg}")
def error(msg) = $stderr.puts("[ERROR] #{msg}")

# Ordena strings por blocos numéricos — ordem natural
# Ex: ["arq_10.pdf", "arq_2.pdf", "arq_1.pdf"] → ["arq_1.pdf", "arq_2.pdf", "arq_10.pdf"]
def natural_sort(arr)
  arr.sort_by { |s| s.scan(/(\d+)|(\D+)/).map { |n, a| n ? n.to_i : a.downcase } }
end

def qpdf_available?
  system('which qpdf > /dev/null 2>&1')
end

def ghostscript_available?
  system('which gs > /dev/null 2>&1')
end

# Mescla uma lista de PDFs em um único arquivo usando qpdf
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

# Comprime um PDF usando GhostScript preservando qualidade de imagem
def compress_pdf(input_path, output_path)
  unless ghostscript_available?
    FileUtils.cp(input_path, output_path)
    return
  end

  cmd = [
    'gs',
    '-sDEVICE=pdfwrite',
    '-dCompatibilityLevel=1.5',
    "-dPDFSETTINGS=/#{GS_QUALITY}",
    '-dNOPAUSE',
    '-dQUIET',
    '-dBATCH',
    '-dColorImageResolution=150',
    '-dGrayImageResolution=150',
    '-dMonoImageResolution=300',
    "-sOutputFile=#{output_path}",
    input_path
  ]

  _, stderr, status = Open3.capture3(*cmd)

  if status.success? && File.exist?(output_path)
    original_kb  = (File.size(input_path)  / 1024.0).round(1)
    compressed_kb = (File.size(output_path) / 1024.0).round(1)
    savings = ((1 - compressed_kb.to_f / original_kb) * 100).round(1)
    log "  Compressão: #{original_kb} KB → #{compressed_kb} KB (#{savings}% menor)"
  else
    warn "  GhostScript falhou, usando PDF sem compressão"
    warn "  #{stderr.strip}" unless stderr.strip.empty?
    FileUtils.cp(input_path, output_path)
  end
end

# ─── Lógica principal ─────────────────────────────────────────────────────────

def process_id_folder(id_folder, output_dir)
  id_name = File.basename(id_folder)
  log "Processando: #{id_name}"

  # Coletar todos os PDFs da pasta em ordem natural
  pdfs = Dir.glob(File.join(id_folder, '*.pdf')) +
         Dir.glob(File.join(id_folder, '*.PDF'))
  pdfs = natural_sort(pdfs)

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

# ─── CLI ──────────────────────────────────────────────────────────────────────

options = { output: nil }

parser = OptionParser.new do |opts|
  opts.banner = <<~BANNER
    Uso: ruby #{File.basename($PROGRAM_NAME)} [opções] <caminho/Evidências>

    Para cada subpasta (ID_NUMERO) dentro de Evidências:
      - Lista os PDFs em ordem natural
      - Junta todos em um PDF único
      - Comprime preservando qualidade de imagem

    Estrutura esperada:
      Evidências/
        ID_0001/
          arquivo1.pdf
          arquivo2.pdf
        ID_0002/
          ...

  BANNER

  opts.on('-o', '--output DIR',
          'Pasta de saída (padrão: subpasta "merged" dentro de Evidências)') do |dir|
    options[:output] = dir
  end

  opts.on('-h', '--help', 'Exibe esta ajuda') do
    puts opts
    exit
  end
end

parser.parse!

if ARGV.empty?
  error 'Informe o caminho da pasta Evidências.'
  puts parser
  exit 1
end

evidencias_path = File.expand_path(ARGV[0])

unless Dir.exist?(evidencias_path)
  error "Pasta não encontrada: #{evidencias_path}"
  exit 1
end

unless qpdf_available?
  error 'qpdf não encontrado. Instale com: sudo apt install qpdf'
  exit 1
end

output_dir = options[:output] || File.join(evidencias_path, 'merged')
FileUtils.mkdir_p(output_dir)

# Subpastas diretas de Evidências (excluindo a própria pasta de saída)
id_folders = Dir.glob(File.join(evidencias_path, '*/'))
             .select { |f| File.directory?(f) }
             .reject { |f| File.expand_path(f) == File.expand_path(output_dir) }

id_folders = natural_sort(id_folders)

if id_folders.empty?
  error "Nenhuma subpasta encontrada em: #{evidencias_path}"
  exit 1
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
