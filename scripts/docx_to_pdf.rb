#!/usr/bin/env ruby
# frozen_string_literal: true

# docx_to_pdf.rb
# Converte arquivos .docx para PDF usando LibreOffice
#
# Uso:
#   ruby docx_to_pdf.rb arquivo.docx
#   ruby docx_to_pdf.rb arquivo.docx --output /caminho/saida/
#   ruby docx_to_pdf.rb *.docx
#   ruby docx_to_pdf.rb --help

require 'optparse'
require 'fileutils'
require 'tmpdir'

# ─── Configurações ────────────────────────────────────────────────────────────

SOFFICE_HELPER = File.expand_path(
  File.join(__dir__, '../mnt/skills/public/docx/scripts/office/soffice.py')
)

# ─── Helpers ──────────────────────────────────────────────────────────────────

def log(msg)   = puts("[INFO]  #{msg}")
def ok(msg)    = puts("[  OK] #{msg}")
def warn(msg)  = $stderr.puts("[WARN]  #{msg}")
def error(msg) = $stderr.puts("[ERROR] #{msg}")

def libreoffice_available?
  system('which soffice > /dev/null 2>&1') ||
    system('which libreoffice > /dev/null 2>&1')
end

# Retorna o comando base do LibreOffice considerando o ambiente sandbox
def soffice_cmd
  if File.exist?(SOFFICE_HELPER)
    ['python3', SOFFICE_HELPER]
  else
    ['soffice']
  end
end

# Converte um único arquivo .docx para PDF
# Retorna o caminho do PDF gerado, ou nil em caso de falha
def convert(input_path, output_dir)
  input_path = File.expand_path(input_path)

  unless File.exist?(input_path)
    error "Arquivo não encontrado: #{input_path}"
    return nil
  end

  unless File.extname(input_path).downcase == '.docx'
    warn "Ignorando (não é .docx): #{input_path}"
    return nil
  end

  # LibreOffice salva o PDF no mesmo diretório do arquivo de entrada,
  # então convertemos em um diretório temporário e depois movemos.
  Dir.mktmpdir('docx_to_pdf_') do |tmp|
    tmp_input = File.join(tmp, File.basename(input_path))
    FileUtils.cp(input_path, tmp_input)

    cmd = soffice_cmd + ['--headless', '--convert-to', 'pdf', '--outdir', tmp, tmp_input]

    log "Convertendo: #{File.basename(input_path)}"
    success = system(*cmd)

    unless success
      error "LibreOffice falhou para: #{input_path}"
      return nil
    end

    tmp_pdf = File.join(tmp, File.basename(input_path, '.docx') + '.pdf')

    unless File.exist?(tmp_pdf)
      error "PDF não gerado para: #{input_path}"
      return nil
    end

    dest = File.join(output_dir, File.basename(tmp_pdf))
    FileUtils.mv(tmp_pdf, dest)
    ok "Gerado: #{dest}"
    dest
  end
end

# ─── CLI ──────────────────────────────────────────────────────────────────────

options = { output: nil }

parser = OptionParser.new do |opts|
  opts.banner = <<~BANNER
    Uso: ruby #{File.basename($PROGRAM_NAME)} [opções] arquivo.docx [arquivo2.docx ...]

    Converte arquivos .docx para PDF usando LibreOffice.

  BANNER

  opts.on('-o', '--output DIR', 'Diretório de saída (padrão: mesmo do arquivo de entrada)') do |dir|
    options[:output] = dir
  end

  opts.on('-h', '--help', 'Exibe esta ajuda') do
    puts opts
    exit
  end
end

parser.parse!

if ARGV.empty?
  error 'Nenhum arquivo especificado.'
  puts parser
  exit 1
end

unless libreoffice_available?
  error 'LibreOffice não encontrado. Instale com: sudo apt install libreoffice'
  exit 1
end

# ─── Execução ─────────────────────────────────────────────────────────────────

results = { ok: [], fail: [] }

ARGV.each do |pattern|
  # Suporta glob passado via argumento (ex: *.docx em sistemas que não expandem)
  files = Dir.glob(pattern)
  files = [pattern] if files.empty?

  files.each do |file|
    output_dir = options[:output] || File.dirname(File.expand_path(file))
    FileUtils.mkdir_p(output_dir)

    pdf = convert(file, output_dir)
    pdf ? results[:ok] << pdf : results[:fail] << file
  end
end

# ─── Resumo ───────────────────────────────────────────────────────────────────

puts
puts '─' * 50
puts "Convertidos com sucesso : #{results[:ok].size}"
puts "Falhas                  : #{results[:fail].size}"

unless results[:fail].empty?
  puts "\nArquivos com falha:"
  results[:fail].each { |f| puts "  • #{f}" }
end

exit(results[:fail].empty? ? 0 : 1)
