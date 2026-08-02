#!/usr/bin/env ruby
# frozen_string_literal: true

# Gera o CHANGELOG.md a partir das mensagens de commit (Conventional Commits)
# e calcula a proxima versao segundo Semantic Versioning.
#
# Uso:
#   ruby scripts/changelog.rb previa                    mostra o que entraria na proxima versao
#   ruby scripts/changelog.rb plano                     imprime chave=valor (bump, versao, mudancas, tag_existe)
#   ruby scripts/changelog.rb gerar 1.2.0 [AAAA-MM-DD]  regrava a regiao gerada do CHANGELOG.md
#   ruby scripts/changelog.rb aplicar-versao 1.2.0      atualiza Cargo.toml e Cargo.lock
#
# Usa apenas a biblioteca padrao do Ruby e o git: nao ha gems para instalar.

require "date"

# O runner pode subir sem locale definido; sem isso, ler acentos quebra.
Encoding.default_external = Encoding::UTF_8

RAIZ = File.expand_path("..", __dir__)
CHANGELOG = File.join(RAIZ, "CHANGELOG.md")
CARGO_TOML = File.join(RAIZ, "Cargo.toml")
CARGO_LOCK = File.join(RAIZ, "Cargo.lock")

MARCA_INICIO = "<!-- changelog:inicio -->"
MARCA_FIM = "<!-- changelog:fim -->"

# Ordem em que as secoes aparecem dentro de uma versao.
SECOES = [
  [:quebra,  "Mudanças importantes"],
  [:feat,    "Novidades"],
  [:fix,     "Correções"],
  [:docs,    "Documentação"],
  [:interno, "Manutenção interna"],
  [:outros,  "Outras alterações"]
].freeze

# `refact` nao e um tipo do Conventional Commits, mas ja foi usado no historico
# deste repositorio; tratamos como sinonimo de `refactor`.
TIPOS_INTERNOS = %w[chore refactor refact build ci test perf style revert].freeze

Commit = Struct.new(:hash, :tipo, :escopo, :quebra, :descricao)

def git(*args)
  saida = IO.popen(["git", "-C", RAIZ, *args], &:read)
  abort("erro ao executar: git #{args.join(' ')}") unless $?.success?
  saida
end

def versao_atual
  conteudo = File.read(CARGO_TOML)
  casamento = conteudo.match(/\[package\].*?^version\s*=\s*"([^"]+)"/m)
  abort("nao encontrei `version` na secao [package] do Cargo.toml") unless casamento
  casamento[1]
end

def nome_pacote
  conteudo = File.read(CARGO_TOML)
  casamento = conteudo.match(/\[package\].*?^name\s*=\s*"([^"]+)"/m)
  abort("nao encontrei `name` na secao [package] do Cargo.toml") unless casamento
  casamento[1]
end

# Versoes ja documentadas no CHANGELOG, da mais recente para a mais antiga.
def versoes_documentadas
  return [] unless File.exist?(CHANGELOG)

  File.read(CHANGELOG).scan(/^## (\d+\.\d+\.\d+)/).flatten
end

def ultima_tag
  git("tag", "--list", "v*", "--sort=-v:refname").split("\n").map(&:strip).reject(&:empty?).first
end

# Sem tag base nao da para saber onde termina o historico ja publicado. Retorna
# a orientacao para quem for ativar a pipeline, ou nil se estiver tudo certo.
def pendencia_de_tag
  return nil if ultima_tag

  documentadas = versoes_documentadas
  return nil if documentadas.empty?

  <<~MSG
    Nenhuma tag de versao encontrada, mas o CHANGELOG ja documenta a versao #{documentadas.first}.
    Sem a tag base o script nao sabe onde termina o historico ja publicado.

    Crie a tag apontando para o ultimo commit dessa versao:
      git tag -a v#{documentadas.first} -m "v#{documentadas.first}" <commit>
      git push origin v#{documentadas.first}
  MSG
end

# Intervalo de commits ainda nao publicados.
def intervalo
  tag = ultima_tag
  return "#{tag}..HEAD" if tag

  pendente = pendencia_de_tag
  abort(pendente) if pendente

  "HEAD"
end

def commits
  separador_registro = "\x1e"
  separador_campo = "\x1f"
  formato = ["%H", "%s", "%b"].join(separador_campo) + separador_registro

  saida = git("log", "--no-merges", "--format=#{formato}", intervalo)

  saida.split(separador_registro).map(&:strip).reject(&:empty?).filter_map do |registro|
    hash, assunto, corpo = registro.split(separador_campo)
    assunto = assunto.to_s.strip
    corpo = corpo.to_s

    # Commits gerados pela propria pipeline nao viram entrada.
    next if assunto.start_with?("chore(release)") || assunto.include?("[skip ci]")

    casamento = assunto.match(/\A(?<tipo>[a-zA-Z]+)(?:\((?<escopo>[^)]*)\))?(?<quebra>!)?:\s*(?<desc>.+)\z/)
    quebra = !casamento.nil? && !casamento[:quebra].nil?
    quebra ||= corpo.match?(/^BREAKING[ -]CHANGE:/)

    Commit.new(
      hash,
      casamento && casamento[:tipo].downcase,
      casamento && casamento[:escopo],
      quebra,
      casamento ? casamento[:desc].strip : assunto
    )
  end
end

def secao_do(commit)
  return :quebra if commit.quebra

  case commit.tipo
  when "feat" then :feat
  when "fix" then :fix
  when "docs" then :docs
  when *TIPOS_INTERNOS then :interno
  else :outros
  end
end

def agrupar(lista)
  lista
    .uniq { |commit| [secao_do(commit), commit.escopo, commit.descricao] }
    .group_by { |commit| secao_do(commit) }
end

def entrada(commit)
  descricao = commit.descricao.sub(/\.\z/, "").strip
  descricao = descricao[0].upcase + descricao[1..].to_s
  prefixo = commit.escopo.to_s.empty? ? "" : "**#{commit.escopo}**: "
  "- #{prefixo}#{descricao} (`#{commit.hash[0, 7]}`)"
end

def corpo_das_secoes(lista)
  grupos = agrupar(lista)

  SECOES.filter_map do |chave, titulo|
    next unless grupos[chave]

    ["### #{titulo}", "", *grupos[chave].map { |commit| entrada(commit) }].join("\n")
  end.join("\n\n")
end

def calcular_bump(lista)
  forcado = ENV["VERSION_BUMP"].to_s.strip.downcase
  return forcado if %w[major minor patch].include?(forcado)

  return "major" if lista.any?(&:quebra)
  return "minor" if lista.any? { |commit| commit.tipo == "feat" }

  "patch"
end

def proxima_versao(atual, bump)
  major, minor, patch = atual.split(".").map(&:to_i)

  case bump
  when "major" then "#{major + 1}.0.0"
  when "minor" then "#{major}.#{minor + 1}.0"
  else "#{major}.#{minor}.#{patch + 1}"
  end
end

def comando_previa
  # A previa e informativa: nao derruba a validacao das PRs enquanto a pipeline
  # nao tiver sido ativada. Ja `plano` e `gerar`, que publicam, sao estritos.
  pendente = pendencia_de_tag
  if pendente
    puts "Previa indisponivel — a pipeline ainda nao foi ativada."
    puts
    puts pendente
    return
  end

  lista = commits

  if lista.empty?
    puts "Nenhuma mudanca para publicar desde #{ultima_tag || 'o inicio do historico'}."
    return
  end

  bump = calcular_bump(lista)
  puts "Intervalo analisado: #{intervalo}"
  puts "Incremento: #{bump} (#{versao_atual} -> #{proxima_versao(versao_atual, bump)})"
  puts
  puts corpo_das_secoes(lista)
  puts

  fora_do_padrao = lista.count { |commit| secao_do(commit) == :outros }
  return if fora_do_padrao.zero?

  puts "Aviso: #{fora_do_padrao} commit(s) cairam em \"Outras alteracoes\" por nao seguirem o padrao de commit."
end

def comando_plano
  lista = commits
  atual = versao_atual
  bump = calcular_bump(lista)
  versao = lista.empty? ? atual : proxima_versao(atual, bump)
  tag_existe = !git("tag", "--list", "v#{versao}").strip.empty?

  puts "bump=#{bump}"
  puts "versao=#{versao}"
  puts "mudancas=#{lista.size}"
  puts "tag_existe=#{tag_existe}"
end

def comando_gerar(versao, data)
  abort("versao invalida: #{versao}") unless versao.to_s.match?(/\A\d+\.\d+\.\d+\z/)

  data ||= Date.today.to_s
  lista = commits
  abort("nenhuma mudanca para publicar em #{versao}") if lista.empty?

  bloco = "## #{versao} — #{data}\n\n#{corpo_das_secoes(lista)}\n\n---\n"

  conteudo = File.read(CHANGELOG)
  inicio = conteudo.index(MARCA_INICIO)
  fim = conteudo.index(MARCA_FIM)
  abort("marcadores #{MARCA_INICIO} / #{MARCA_FIM} nao encontrados no CHANGELOG.md") unless inicio && fim

  antes = conteudo[0...inicio]
  regiao = conteudo[(inicio + MARCA_INICIO.length)...fim]
  depois = conteudo[fim..]

  # Preserva as versoes ja geradas; a secao "Nao publicado" e zerada porque o
  # que estava nela acabou de entrar na versao publicada.
  versoes_anteriores = regiao[/^## \d+\.\d+\.\d+.*/m].to_s.rstrip

  nova_regiao = +"\n\n"
  nova_regiao << "## Não publicado\n\nNada por enquanto.\n\n---\n\n"
  nova_regiao << bloco
  nova_regiao << "\n#{versoes_anteriores}\n" unless versoes_anteriores.empty?
  nova_regiao << "\n"

  File.write(CHANGELOG, "#{antes}#{MARCA_INICIO}#{nova_regiao}#{depois}")
  puts "CHANGELOG.md atualizado com a versao #{versao} (#{lista.size} commit(s))."
end

def comando_aplicar_versao(versao)
  abort("versao invalida: #{versao}") unless versao.to_s.match?(/\A\d+\.\d+\.\d+\z/)

  pacote = nome_pacote

  toml = File.read(CARGO_TOML)
  toml = toml.sub(/(\[package\].*?^version\s*=\s*")[^"]+(")/m, "\\1#{versao}\\2")
  File.write(CARGO_TOML, toml)

  if File.exist?(CARGO_LOCK)
    lock = File.read(CARGO_LOCK)
    padrao = /(\[\[package\]\]\nname = "#{Regexp.escape(pacote)}"\nversion = ")[^"]+(")/
    abort("nao encontrei o pacote #{pacote} no Cargo.lock") unless lock.match?(padrao)

    File.write(CARGO_LOCK, lock.sub(padrao, "\\1#{versao}\\2"))
  end

  puts "Versao #{versao} aplicada em Cargo.toml e Cargo.lock."
end

def ajuda
  texto = File.readlines(__FILE__, encoding: "UTF-8")
              .drop(2) # shebang e frozen_string_literal
              .take_while { |linha| linha.start_with?("#") || linha.strip.empty? }
              .map { |linha| linha.sub(/\A# ?/, "") }
              .join
              .strip

  puts texto
end

comando, *argumentos = ARGV

case comando
when "previa" then comando_previa
when "plano" then comando_plano
when "gerar" then comando_gerar(argumentos[0], argumentos[1])
when "aplicar-versao" then comando_aplicar_versao(argumentos[0])
when nil, "ajuda", "-h", "--help" then ajuda
else
  warn("comando desconhecido: #{comando}")
  ajuda
  exit 1
end
