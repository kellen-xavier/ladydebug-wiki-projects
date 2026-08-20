#!/usr/bin/env python3
"""Gera imagem a partir de uma extracao de PDF -- e somente dela.

Entrada: a pasta produzida por `extrair_spec_pdf.py` (spec.json + texto.txt).
O script monta o briefing usando apenas valores presentes na extracao, valida
cada valor contra o texto extraido e so entao:

  * desenha a "ficha visual" (SVG deterministico, sem IA, sem rede); e/ou
  * chama o agente de imagem configurado localmente pelo usuario.

Uso:
    python3 scripts/gerar_imagem_spec.py knowledge/extracted/spec-orion
    python3 scripts/gerar_imagem_spec.py .../spec.json --tipo ilustracao --agent meu-agent
    python3 scripts/gerar_imagem_spec.py .../spec-orion --listar-agents

Sem dependencias externas: apenas a biblioteca padrao do Python 3.8+.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
import unicodedata
from datetime import datetime, timezone
from pathlib import Path

VERSAO = "1.0.0"

# Unicas frases que o briefing pode conter alem dos valores extraidos do PDF.
# Qualquer outra linha reprova na validacao -- e assim que "somente o que foi
# extraido" deixa de ser promessa e vira verificacao.
MOLDE = [
    "Gere uma imagem usando exclusivamente os dados listados abaixo.",
    "Os dados foram extraidos automaticamente de um PDF de especificacao.",
    "Nao acrescente objeto, texto, marca, cenario ou detalhe que nao esteja na lista.",
    "Atributo ausente na lista deve ficar neutro ou fora do enquadramento; nunca inventado.",
    "Respeite proporcoes, cores e materiais exatamente como descritos.",
    "Fundo neutro e uniforme, sem elementos decorativos adicionais.",
    "DADOS EXTRAIDOS",
    "ATRIBUTOS VISUAIS EXPLICITOS",
    "OBSERVACOES DO DOCUMENTO",
]

CONFIG_EXEMPLO = {
    "agent_padrao": "meu-agent",
    "agents": {
        "meu-agent": {
            "descricao": "gerador de imagem configurado na maquina do usuario",
            "comando": ["meu-cli", "--prompt-file", "{prompt_arquivo}", "--out", "{saida}"],
            "prompt_via": "arquivo",
            "saida_via": "arquivo",
            "timeout_s": 300,
        }
    },
}


# --------------------------------------------------------------------------
# Utilidades
# --------------------------------------------------------------------------
def normalizar(texto: str) -> str:
    texto = unicodedata.normalize("NFKD", texto)
    texto = "".join(c for c in texto if not unicodedata.combining(c))
    return re.sub(r"\s+", " ", texto).strip().lower()


def escapar_xml(texto: str) -> str:
    return (
        texto.replace("&", "&amp;")
        .replace("<", "&lt;")
        .replace(">", "&gt;")
        .replace('"', "&quot;")
    )


def quebrar(texto: str, largura_px: float, tamanho_fonte: float) -> list:
    """Quebra o texto em linhas que caibam na largura, sem cortar palavras."""
    por_caractere = tamanho_fonte * 0.55
    maximo = max(8, int(largura_px / por_caractere))
    linhas, atual = [], ""
    for palavra in texto.split():
        candidata = f"{atual} {palavra}".strip()
        if len(candidata) <= maximo:
            atual = candidata
            continue
        if atual:
            linhas.append(atual)
        while len(palavra) > maximo:
            linhas.append(palavra[: maximo - 1] + "-")
            palavra = palavra[maximo - 1 :]
        atual = palavra
    if atual:
        linhas.append(atual)
    return linhas or [""]


def carregar_extracao(alvo: Path) -> tuple:
    spec_json = alvo if alvo.is_file() else alvo / "spec.json"
    if not spec_json.is_file():
        raise SystemExit(f"[ERRO] spec.json nao encontrado em: {alvo}")
    spec = json.loads(spec_json.read_text(encoding="utf-8"))
    texto_txt = spec_json.parent / "texto.txt"
    if not texto_txt.is_file():
        raise SystemExit(f"[ERRO] texto.txt nao encontrado ao lado de {spec_json} (necessario para validar).")
    return spec, texto_txt.read_text(encoding="utf-8"), spec_json.parent


# --------------------------------------------------------------------------
# Briefing: montado so com valores extraidos
# --------------------------------------------------------------------------
def montar_briefing(spec: dict, max_campos: int) -> dict:
    doc = spec.get("documento", {})
    titulo = spec.get("titulos", [{}])[0].get("texto") if spec.get("titulos") else None

    campos = spec.get("campos", [])[:max_campos]
    usados_texto = {normalizar(f"{c['chave']}: {c['valor']}") for c in campos}
    visuais = [
        v
        for v in spec.get("atributos_visuais", [])
        if normalizar(v["texto"]) not in usados_texto and (v.get("unidades") or v.get("valor"))
    ][:max_campos]
    observacoes = spec.get("listas", [])[:max_campos]

    itens = []  # (secao, texto_limpo, procedencia)
    if titulo:
        itens.append(("titulo", titulo, spec["titulos"][0]))
    for campo in campos:
        itens.append(("campo", f"{campo['chave']}: {campo['valor']}", campo))
    for visual in visuais:
        itens.append(("visual", visual["texto"], visual))
    for obs in observacoes:
        itens.append(("observacao", obs["texto"], obs))

    return {
        "documento": doc,
        "titulo": titulo,
        "itens": itens,
        "campos": campos,
        # visuais: so o que ainda nao apareceu nos campos, para nao repetir no prompt
        "visuais": visuais,
        # visuais_todos: tudo que tem medida, cor ou material, usado na ficha
        "visuais_todos": [v for v in spec.get("atributos_visuais", []) if v.get("unidades")][:max_campos],
        "observacoes": observacoes,
    }


def texto_do_prompt(briefing: dict) -> str:
    linhas = MOLDE[:6] + [""]
    if briefing["titulo"]:
        linhas += [f"DADOS EXTRAIDOS", f"- {briefing['titulo']}"]
    else:
        linhas += ["DADOS EXTRAIDOS"]
    for campo in briefing["campos"]:
        linhas.append(f"- {campo['chave']}: {campo['valor']}")
    if briefing["visuais"]:
        linhas += ["", "ATRIBUTOS VISUAIS EXPLICITOS"]
        linhas += [f"- {v['texto']}" for v in briefing["visuais"]]
    if briefing["observacoes"]:
        linhas += ["", "OBSERVACOES DO DOCUMENTO"]
        linhas += [f"- {o['texto']}" for o in briefing["observacoes"]]
    return "\n".join(linhas).strip() + "\n"


def validar_prompt(prompt: str, texto_extraido: str) -> list:
    """Confere se cada linha do prompt e molde fixo ou valor presente na extracao."""
    base = normalizar(texto_extraido)
    moldes = {normalizar(m) for m in MOLDE}
    reprovadas = []
    for numero, linha in enumerate(prompt.splitlines(), start=1):
        limpa = linha.strip()
        if not limpa:
            continue
        alvo = limpa[1:].strip() if limpa.startswith("- ") or limpa.startswith("-") else limpa
        alvo_norm = normalizar(alvo)
        if not alvo_norm or alvo_norm in moldes:
            continue
        if alvo_norm in base:
            continue
        # valores vindos de linhas com varios rotulos: valida chave e valor separados
        if ":" in alvo:
            chave, _, valor = alvo.partition(":")
            if normalizar(chave) in base and normalizar(valor) in base:
                continue
        reprovadas.append((numero, limpa))
    return reprovadas


def escrever_briefing_md(briefing: dict, prompt: str, spec_hash: str) -> str:
    doc = briefing["documento"]
    linhas = [
        f"# Briefing de imagem: {doc.get('arquivo', 'documento')}",
        "",
        "> Montado exclusivamente com valores extraidos do PDF. Cada item indica",
        "> pagina e linha de origem. Nada aqui foi interpretado ou complementado.",
        "",
        "| Metadado | Valor |",
        "| --- | --- |",
        f"| PDF de origem | `{doc.get('arquivo', '-')}` |",
        f"| SHA-256 do PDF | `{doc.get('sha256', '-')}` |",
        f"| SHA-256 do spec.json | `{spec_hash}` |",
        f"| Paginas | {doc.get('paginas', '-')} |",
        f"| Extraido em | {doc.get('extraido_em', '-')} |",
        f"| Briefing gerado em | {datetime.now(timezone.utc).astimezone().isoformat(timespec='seconds')} |",
        "",
        "## Itens do briefing e sua origem",
        "",
        "| Secao | Conteudo | Pagina | Linha |",
        "| --- | --- | --- | --- |",
    ]
    for secao, conteudo, origem in briefing["itens"]:
        linhas.append(
            f"| {secao} | {conteudo.replace('|', chr(92) + '|')} | {origem.get('pagina', '-')} | {origem.get('linha', '-')} |"
        )
    if not briefing["itens"]:
        linhas.append("| - | pendente-validacao: nada extraivel para briefing | - | - |")
    linhas += [
        "",
        "## Prompt enviado ao agente (integral)",
        "",
        "```text",
        prompt.rstrip(),
        "```",
        "",
    ]
    return "\n".join(linhas)


# --------------------------------------------------------------------------
# Ficha visual (SVG deterministico)
# --------------------------------------------------------------------------
def renderizar_ficha(briefing: dict, largura: int = 1000) -> str:
    doc = briefing["documento"]
    margem = 48
    col_chave = 300
    fonte_valor = 16
    fonte_chave = 16
    interlinha = 22

    titulo = briefing["titulo"] or doc.get("arquivo", "Ficha visual")
    linhas_titulo = quebrar(titulo, largura - 2 * margem, 28)

    linhas_campo = []
    for campo in briefing["campos"] or [{"chave": "pendente-validacao", "valor": "nenhum campo extraido do PDF", "pagina": "-", "linha": "-"}]:
        chave = quebrar(str(campo["chave"]), col_chave - 24, fonte_chave)
        valor = quebrar(str(campo["valor"]), largura - 2 * margem - col_chave - 110, fonte_valor)
        linhas_campo.append((chave, valor, campo.get("pagina", "-"), campo.get("linha", "-")))

    itens_visuais = [v["texto"] for v in briefing.get("visuais_todos") or briefing["visuais"]]
    linhas_visuais = [quebrar(t, largura - 2 * margem - 24, fonte_valor) for t in itens_visuais]

    y = margem + 28 * len(linhas_titulo) + 18
    y_meta = y
    y += 26 + 30  # metadados + cabecalho da tabela
    y_tabela = y
    alturas = []
    for chave, valor, _p, _l in linhas_campo:
        altura = max(len(chave), len(valor)) * interlinha + 14
        alturas.append(altura)
        y += altura
    y_visuais = y + 34
    y = y_visuais + 26 + sum(len(l) * interlinha + 8 for l in linhas_visuais)
    y_rodape = y + 24
    altura_total = int(y_rodape + 74)

    p = []
    a = p.append
    a(f'<svg xmlns="http://www.w3.org/2000/svg" width="{largura}" height="{altura_total}" viewBox="0 0 {largura} {altura_total}" font-family="Helvetica, Arial, sans-serif">')
    a(f'<rect width="{largura}" height="{altura_total}" fill="#ffffff"/>')
    a(f'<rect x="0" y="0" width="{largura}" height="10" fill="#1f3a5f"/>')

    ty = margem + 22
    for linha in linhas_titulo:
        a(f'<text x="{margem}" y="{ty}" font-size="28" font-weight="bold" fill="#12263a">{escapar_xml(linha)}</text>')
        ty += 28
    meta = f"{doc.get('arquivo', '-')}  |  {doc.get('paginas', '-')} pagina(s)  |  extraido em {doc.get('extraido_em', '-')}"
    a(f'<text x="{margem}" y="{y_meta + 6}" font-size="13" fill="#5b6b7c">{escapar_xml(meta)}</text>')
    a(f'<line x1="{margem}" y1="{y_meta + 18}" x2="{largura - margem}" y2="{y_meta + 18}" stroke="#d7dee6" stroke-width="1"/>')

    a(f'<text x="{margem}" y="{y_tabela - 12}" font-size="13" font-weight="bold" fill="#1f3a5f">CAMPOS EXTRAIDOS DO PDF</text>')
    linha_y = y_tabela
    for indice, ((chave, valor, pagina, linha_origem), altura) in enumerate(zip(linhas_campo, alturas)):
        if indice % 2 == 0:
            a(f'<rect x="{margem}" y="{linha_y}" width="{largura - 2 * margem}" height="{altura}" fill="#f4f7fa"/>')
        ty = linha_y + 20
        for texto in chave:
            a(f'<text x="{margem + 12}" y="{ty}" font-size="{fonte_chave}" font-weight="bold" fill="#334e68">{escapar_xml(texto)}</text>')
            ty += interlinha
        ty = linha_y + 20
        for texto in valor:
            a(f'<text x="{margem + col_chave}" y="{ty}" font-size="{fonte_valor}" fill="#12263a">{escapar_xml(texto)}</text>')
            ty += interlinha
        a(f'<text x="{largura - margem - 12}" y="{linha_y + 20}" font-size="11" fill="#8296aa" text-anchor="end">p.{pagina} l.{linha_origem}</text>')
        linha_y += altura

    a(f'<text x="{margem}" y="{y_visuais}" font-size="13" font-weight="bold" fill="#1f3a5f">ATRIBUTOS VISUAIS EXPLICITOS</text>')
    vy = y_visuais + 26
    if not linhas_visuais:
        a(f'<text x="{margem + 12}" y="{vy}" font-size="{fonte_valor}" fill="#a33">pendente-validacao: nenhuma medida, cor ou material reconhecido</text>')
    for bloco in linhas_visuais:
        a(f'<circle cx="{margem + 5}" cy="{vy - 5}" r="3" fill="#1f3a5f"/>')
        for texto in bloco:
            a(f'<text x="{margem + 18}" y="{vy}" font-size="{fonte_valor}" fill="#12263a">{escapar_xml(texto)}</text>')
            vy += interlinha
        vy += 8

    a(f'<line x1="{margem}" y1="{y_rodape}" x2="{largura - margem}" y2="{y_rodape}" stroke="#d7dee6" stroke-width="1"/>')
    rodape = [
        f"Fonte: {doc.get('arquivo', '-')} (SHA-256 {str(doc.get('sha256', '-'))[:16]}...), motor de extracao: {doc.get('motor', '-')}",
        "Conteudo 100% extraido do PDF. Campos ausentes aparecem como pendente-validacao.",
    ]
    fy = y_rodape + 22
    for texto in rodape:
        a(f'<text x="{margem}" y="{fy}" font-size="12" fill="#5b6b7c">{escapar_xml(texto)}</text>')
        fy += 18
    a("</svg>")
    return "\n".join(p) + "\n"


def converter_svg_para_png(svg: Path, png: Path, largura: int) -> str:
    tentativas = [
        (["rsvg-convert", "-w", str(largura), "-o", str(png), str(svg)], "rsvg-convert"),
        (["inkscape", str(svg), "--export-type=png", f"--export-filename={png}", f"--export-width={largura}"], "inkscape"),
        (["magick", "-density", "150", str(svg), str(png)], "magick"),
        (["convert", "-density", "150", str(svg), str(png)], "convert"),
        (["chromium", "--headless", "--disable-gpu", f"--screenshot={png}", f"--window-size={largura},1400", str(svg)], "chromium"),
    ]
    for comando, nome in tentativas:
        if not shutil.which(comando[0]):
            continue
        try:
            resultado = subprocess.run(comando, capture_output=True, timeout=180)
        except Exception:
            continue
        if resultado.returncode == 0 and png.is_file() and png.stat().st_size > 0:
            return nome
    return ""


# --------------------------------------------------------------------------
# Agente de imagem configurado localmente
# --------------------------------------------------------------------------
def caminhos_config(explicito: str = "") -> list:
    candidatos = []
    if explicito:
        candidatos.append(Path(explicito))
    if os.environ.get("SPEC_IMAGEM_CONFIG"):
        candidatos.append(Path(os.environ["SPEC_IMAGEM_CONFIG"]))
    candidatos.append(Path(".agents/config/imagem-agents.json"))
    candidatos.append(Path.home() / ".config/spec-imagem/agents.json")
    return candidatos


def carregar_config(explicito: str = "") -> tuple:
    for caminho in caminhos_config(explicito):
        if caminho.is_file():
            try:
                return json.loads(caminho.read_text(encoding="utf-8")), caminho
            except json.JSONDecodeError as erro:
                raise SystemExit(f"[ERRO] configuracao invalida em {caminho}: {erro}")
    return {}, None


def chamar_agent(nome: str, config: dict, prompt: str, prompt_arquivo: Path, saida: Path, largura: int, altura: int) -> dict:
    agents = config.get("agents") or {}
    nome = nome or config.get("agent_padrao") or ""
    if not agents:
        raise SystemExit(
            "[ERRO] nenhum agente de imagem configurado.\n"
            "       Crie .agents/config/imagem-agents.json a partir de\n"
            "       .agents/config/imagem-agents.exemplo.json e aponte o comando do seu gerador.\n"
            "       Exemplo minimo:\n" + json.dumps(CONFIG_EXEMPLO, ensure_ascii=False, indent=2)
        )
    if nome not in agents:
        raise SystemExit(f"[ERRO] agente '{nome}' nao existe na configuracao. Disponiveis: {', '.join(sorted(agents))}")

    agent = agents[nome]
    comando_modelo = agent.get("comando")
    if not isinstance(comando_modelo, list) or not comando_modelo:
        raise SystemExit(f"[ERRO] agente '{nome}' sem 'comando' (lista de argumentos) na configuracao.")

    substituicoes = {
        "{prompt}": prompt,
        "{prompt_arquivo}": str(prompt_arquivo),
        "{saida}": str(saida),
        "{largura}": str(largura),
        "{altura}": str(altura),
    }
    comando = []
    for parte in comando_modelo:
        parte = str(parte)
        for chave, valor in substituicoes.items():
            parte = parte.replace(chave, valor)
        comando.append(parte)

    ambiente = os.environ.copy()
    for chave, valor in (agent.get("env") or {}).items():
        ambiente[str(chave)] = str(valor)

    prompt_via = agent.get("prompt_via", "arquivo")
    entrada = prompt.encode("utf-8") if prompt_via == "stdin" else None
    saida_via = agent.get("saida_via", "arquivo")
    timeout = int(agent.get("timeout_s", 300) or 300)

    print(f"[INFO] agente        : {nome}")
    print(f"[INFO] comando       : {' '.join(comando)}")
    try:
        resultado = subprocess.run(comando, input=entrada, capture_output=True, timeout=timeout, env=ambiente)
    except FileNotFoundError:
        raise SystemExit(f"[ERRO] executavel nao encontrado: {comando[0]} (configure o agente na sua maquina)")
    except subprocess.TimeoutExpired:
        raise SystemExit(f"[ERRO] agente '{nome}' excedeu {timeout}s")

    erro_txt = resultado.stderr.decode("utf-8", "replace").strip()
    if resultado.returncode != 0:
        raise SystemExit(f"[ERRO] agente '{nome}' falhou (codigo {resultado.returncode}): {erro_txt[:800]}")

    if saida_via == "stdout":
        if not resultado.stdout:
            raise SystemExit(f"[ERRO] agente '{nome}' nao devolveu bytes na saida padrao.")
        saida.write_bytes(resultado.stdout)
    if not saida.is_file() or saida.stat().st_size == 0:
        raise SystemExit(f"[ERRO] agente '{nome}' nao gravou a imagem em {saida}")

    dados = saida.read_bytes()
    formato = "desconhecido"
    if dados.startswith(b"\x89PNG"):
        formato = "png"
    elif dados.startswith(b"\xff\xd8\xff"):
        formato = "jpeg"
    elif dados[:4] == b"RIFF" and dados[8:12] == b"WEBP":
        formato = "webp"
    elif dados.lstrip()[:5] in (b"<svg ", b"<?xml"):
        formato = "svg"
    if formato != "desconhecido" and saida.suffix.lower().lstrip(".") not in (formato, "jpg" if formato == "jpeg" else formato):
        novo = saida.with_suffix(f".{formato}")
        saida.rename(novo)
        saida = novo

    return {
        "agent": nome,
        "comando": comando,
        "arquivo": str(saida),
        "formato": formato,
        "bytes": len(dados),
        "sha256": hashlib.sha256(dados).hexdigest(),
        "stderr": erro_txt[:2000],
    }


# --------------------------------------------------------------------------
# Principal
# --------------------------------------------------------------------------
def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description="Gera imagem a partir de uma extracao de PDF (e somente dela).")
    parser.add_argument("extracao", nargs="?", help="pasta da extracao ou caminho do spec.json")
    parser.add_argument("-t", "--tipo", choices=["ficha", "ilustracao", "ambos"], default="ambos", help="o que gerar (padrao: ambos)")
    parser.add_argument("-o", "--saida", default="knowledge/curated", help="diretorio base de saida (padrao: knowledge/curated)")
    parser.add_argument("-a", "--agent", default="", help="agente de imagem a usar (padrao: agent_padrao da configuracao)")
    parser.add_argument("-c", "--config", default="", help="arquivo de configuracao dos agentes")
    parser.add_argument("--largura", type=int, default=1024, help="largura da imagem (padrao: 1024)")
    parser.add_argument("--altura", type=int, default=1024, help="altura da imagem (padrao: 1024)")
    parser.add_argument("--max-campos", type=int, default=40, help="maximo de campos levados ao briefing (padrao: 40)")
    parser.add_argument("--dry-run", action="store_true", help="monta e valida o briefing e a ficha, sem chamar o agente")
    parser.add_argument("--listar-agents", action="store_true", help="lista os agentes configurados e sai")
    args = parser.parse_args(argv)

    config, arquivo_config = carregar_config(args.config)
    if args.listar_agents:
        if not config.get("agents"):
            print("Nenhum agente configurado. Procurado em:")
            for caminho in caminhos_config(args.config):
                print(f"  - {caminho}")
            print("\nModelo de configuracao:\n" + json.dumps(CONFIG_EXEMPLO, ensure_ascii=False, indent=2))
            return 1
        print(f"Configuracao: {arquivo_config}")
        print(f"Padrao      : {config.get('agent_padrao', '-')}")
        for nome, agent in sorted(config["agents"].items()):
            print(f"  - {nome}: {agent.get('descricao', '')}")
            print(f"      comando: {' '.join(str(p) for p in agent.get('comando', []))}")
        return 0

    if not args.extracao:
        parser.error("informe a pasta da extracao (ou use --listar-agents)")

    spec, texto_extraido, pasta_extracao = carregar_extracao(Path(args.extracao).expanduser())
    spec_hash = hashlib.sha256((pasta_extracao / "spec.json").read_bytes()).hexdigest()

    briefing = montar_briefing(spec, args.max_campos)
    prompt = texto_do_prompt(briefing)
    reprovadas = validar_prompt(prompt, texto_extraido)
    if reprovadas:
        print("[ERRO] o briefing contem conteudo que NAO esta na extracao do PDF:", file=sys.stderr)
        for numero, linha in reprovadas:
            print(f"       linha {numero}: {linha}", file=sys.stderr)
        return 3

    destino = Path(args.saida) / pasta_extracao.name / "imagem"
    destino.mkdir(parents=True, exist_ok=True)
    prompt_arquivo = destino / "prompt.txt"
    prompt_arquivo.write_text(prompt, encoding="utf-8")
    (destino / "briefing.md").write_text(escrever_briefing_md(briefing, prompt, spec_hash), encoding="utf-8")
    print(f"[  OK] briefing validado: {len(briefing['itens'])} item(ns), todos rastreados no PDF")

    manifesto = {
        "gerado_em": datetime.now(timezone.utc).astimezone().isoformat(timespec="seconds"),
        "versao_gerador": VERSAO,
        "origem": {
            "extracao": str(pasta_extracao),
            "spec_sha256": spec_hash,
            "pdf": spec.get("documento", {}).get("arquivo"),
            "pdf_sha256": spec.get("documento", {}).get("sha256"),
        },
        "briefing": {
            "arquivo": str(prompt_arquivo),
            "itens": [
                {"secao": secao, "conteudo": conteudo, "pagina": origem.get("pagina"), "linha": origem.get("linha")}
                for secao, conteudo, origem in briefing["itens"]
            ],
            "validacao": "aprovado: todo valor do prompt existe em texto.txt",
        },
        "saidas": [],
    }

    if args.tipo in ("ficha", "ambos"):
        svg = destino / "ficha.svg"
        svg.write_text(renderizar_ficha(briefing), encoding="utf-8")
        registro = {"tipo": "ficha", "motor": "render deterministico (sem IA)", "arquivo": str(svg)}
        png = destino / "ficha.png"
        conversor = converter_svg_para_png(svg, png, args.largura)
        if conversor:
            registro["png"] = str(png)
            registro["conversor"] = conversor
            print(f"[  OK] ficha            : {svg} + {png} ({conversor})")
        else:
            print(f"[  OK] ficha            : {svg}")
            print("[AVISO] nenhum conversor SVG->PNG encontrado (rsvg-convert, inkscape, magick, chromium); ficha entregue em SVG.", file=sys.stderr)
        manifesto["saidas"].append(registro)

    if args.tipo in ("ilustracao", "ambos"):
        if args.dry_run:
            print("[INFO] --dry-run: agente de imagem nao foi chamado.")
        else:
            resultado = chamar_agent(
                args.agent, config, prompt, prompt_arquivo, destino / "ilustracao.png", args.largura, args.altura
            )
            resultado["tipo"] = "ilustracao"
            resultado["config"] = str(arquivo_config) if arquivo_config else None
            manifesto["saidas"].append(resultado)
            print(f"[  OK] ilustracao       : {resultado['arquivo']} ({resultado['formato']}, {resultado['bytes']} bytes)")

    (destino / "manifesto.json").write_text(json.dumps(manifesto, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"[  OK] saida            : {destino}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
