#!/usr/bin/env ruby
# frozen_string_literal: true

# Verifica os links internos dos arquivos Markdown do repositorio.
#
# Uso:
#   ruby scripts/verificar-links.rb
#
# Links externos (http, https, mailto) e ancoras puras (#secao) sao ignorados.
# Caminhos iniciados por "/" sao resolvidos a partir da raiz do repositorio;
# os demais, a partir da pasta do arquivo que contem o link.
# Sai com codigo 1 se algum destino nao existir.

RAIZ = File.expand_path("..", __dir__)
IGNORAR = %w[.git target node_modules].freeze
LINK = /(!?)\[[^\]]*\]\(\s*([^)\s]+)(?:\s+"[^"]*")?\s*\)/

def arquivos_markdown
  Dir.glob("**/*.md", base: RAIZ).reject do |caminho|
    IGNORAR.any? { |pasta| caminho.start_with?("#{pasta}/") }
  end.sort
end

def externo?(destino)
  destino.match?(%r{\A(?:[a-z][a-z0-9+.-]*:|//)}i) || destino.start_with?("#")
end

def resolver(arquivo, destino)
  caminho = destino.split("#").first.to_s
  return nil if caminho.empty?

  if caminho.start_with?("/")
    File.join(RAIZ, caminho.delete_prefix("/"))
  else
    File.expand_path(caminho, File.join(RAIZ, File.dirname(arquivo)))
  end
end

quebrados = []
total = 0

arquivos_markdown.each do |arquivo|
  File.readlines(File.join(RAIZ, arquivo), encoding: "UTF-8").each_with_index do |linha, indice|
    linha.scan(LINK) do |_imagem, destino|
      next if externo?(destino)

      alvo = resolver(arquivo, destino)
      next if alvo.nil?

      total += 1
      next if File.exist?(alvo)

      quebrados << "#{arquivo}:#{indice + 1}: #{destino}"
    end
  end
end

if quebrados.empty?
  puts "Links internos verificados: #{total} em #{arquivos_markdown.size} arquivo(s). Nenhum quebrado."
  exit 0
end

warn "Links internos quebrados (#{quebrados.size} de #{total}):"
quebrados.each { |item| warn "  #{item}" }
exit 1
