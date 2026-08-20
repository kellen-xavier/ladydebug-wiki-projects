#!/usr/bin/env python3
"""Extrai o conteudo de um PDF de especificacoes de forma deterministica.

O script NAO interpreta o documento: ele apenas le a camada de texto embutida
no PDF e grava o resultado com rastreabilidade (pagina e linha) de cada trecho.
Nenhum conteudo e inventado, resumido ou reescrito.

Uso:
    python3 scripts/extrair_spec_pdf.py especificacao.pdf
    python3 scripts/extrair_spec_pdf.py especificacao.pdf -o knowledge/extracted --preservar

Saida (em knowledge/extracted/<slug>/):
    texto.txt   texto cru, com marcador de pagina
    spec.json   estrutura com campos, titulos, listas e atributos visuais
    spec.md     leitura humana da extracao, com fonte de cada item

Sem dependencias externas: apenas a biblioteca padrao do Python 3.8+.
"""

from __future__ import annotations

import argparse
import binascii
import hashlib
import json
import re
import shutil
import subprocess
import sys
import unicodedata
import zlib
from datetime import datetime, timezone
from pathlib import Path

VERSAO = "1.0.0"

WS = b"\x00\t\n\x0c\r "
DELIM = b"()<>[]{}/%"


# --------------------------------------------------------------------------
# Tipos basicos do PDF
# --------------------------------------------------------------------------
class Name(str):
    """Nome PDF (/Type). Subclasse de str para uso direto como chave."""


class Ref:
    __slots__ = ("num", "gen")

    def __init__(self, num: int, gen: int):
        self.num = num
        self.gen = gen

    def __repr__(self) -> str:  # pragma: no cover - depuracao
        return f"Ref({self.num},{self.gen})"


class Stream:
    __slots__ = ("dic", "raw")

    def __init__(self, dic: dict, raw: bytes):
        self.dic = dic
        self.raw = raw


# --------------------------------------------------------------------------
# Leitor de sintaxe PDF
# --------------------------------------------------------------------------
class Lexer:
    def __init__(self, buf: bytes, pos: int = 0):
        self.buf = buf
        self.pos = pos

    def skip_ws(self) -> None:
        buf, n = self.buf, len(self.buf)
        while self.pos < n:
            c = buf[self.pos]
            if c in WS:
                self.pos += 1
            elif c == 0x25:  # '%' comentario
                while self.pos < n and buf[self.pos] not in b"\r\n":
                    self.pos += 1
            else:
                break

    def peek_keyword(self) -> bytes:
        self.skip_ws()
        m = re.compile(rb"[A-Za-z'\"*]+").match(self.buf, self.pos)
        return m.group(0) if m else b""

    def parse(self):
        self.skip_ws()
        buf, n = self.buf, len(self.buf)
        if self.pos >= n:
            return None
        c = buf[self.pos]

        if c == 0x2F:  # '/'
            return self._name()
        if c == 0x28:  # '('
            return self._literal_string()
        if c == 0x3C:  # '<'
            if buf[self.pos : self.pos + 2] == b"<<":
                return self._dict()
            return self._hex_string()
        if c == 0x5B:  # '['
            self.pos += 1
            arr = []
            while True:
                self.skip_ws()
                if self.pos >= n:
                    break
                if buf[self.pos] == 0x5D:  # ']'
                    self.pos += 1
                    break
                antes = self.pos
                arr.append(self.parse())
                if self.pos == antes:  # nao avancou: aborta para nao travar
                    self.pos += 1
            return arr
        if c == 0x5D or c == 0x3E or c == 0x29:  # fecha solto
            self.pos += 1
            return None

        m = re.compile(rb"[+-]?(?:\d+\.\d*|\.\d+|\d+)").match(buf, self.pos)
        if m:
            return self._number_or_ref(m)

        kw = re.compile(rb"[A-Za-z]+").match(buf, self.pos)
        if kw:
            self.pos = kw.end()
            palavra = kw.group(0)
            if palavra == b"true":
                return True
            if palavra == b"false":
                return False
            if palavra == b"null":
                return None
            return Name(palavra.decode("latin-1"))
        self.pos += 1
        return None

    def _number_or_ref(self, m):
        texto = m.group(0)
        fim = m.end()
        if b"." not in texto:
            ref = re.compile(rb"\s+(\d+)\s+R\b").match(self.buf, fim)
            if ref:
                self.pos = ref.end()
                return Ref(int(texto), int(ref.group(1)))
        self.pos = fim
        return float(texto) if b"." in texto else int(texto)

    def _name(self) -> Name:
        self.pos += 1
        buf, n = self.buf, len(self.buf)
        ini = self.pos
        while self.pos < n and buf[self.pos] not in WS and buf[self.pos] not in DELIM:
            self.pos += 1
        bruto = buf[ini : self.pos]
        if b"#" in bruto:
            bruto = re.sub(rb"#([0-9A-Fa-f]{2})", lambda m: bytes([int(m.group(1), 16)]), bruto)
        return Name(bruto.decode("latin-1"))

    def _literal_string(self) -> bytes:
        self.pos += 1
        buf, n = self.buf, len(self.buf)
        saida = bytearray()
        nivel = 1
        while self.pos < n:
            c = buf[self.pos]
            if c == 0x5C:  # '\'
                self.pos += 1
                if self.pos >= n:
                    break
                e = buf[self.pos]
                mapa = {0x6E: 10, 0x72: 13, 0x74: 9, 0x62: 8, 0x66: 12}
                if e in mapa:
                    saida.append(mapa[e])
                    self.pos += 1
                elif 0x30 <= e <= 0x37:  # octal
                    oct_digits = b""
                    for _ in range(3):
                        if self.pos < n and 0x30 <= buf[self.pos] <= 0x37:
                            oct_digits += buf[self.pos : self.pos + 1]
                            self.pos += 1
                        else:
                            break
                    saida.append(int(oct_digits, 8) & 0xFF)
                elif e in b"\r\n":  # quebra de linha escapada
                    self.pos += 1
                    if e == 13 and self.pos < n and buf[self.pos] == 10:
                        self.pos += 1
                else:
                    saida.append(e)
                    self.pos += 1
                continue
            if c == 0x28:
                nivel += 1
            elif c == 0x29:
                nivel -= 1
                if nivel == 0:
                    self.pos += 1
                    break
            saida.append(c)
            self.pos += 1
        return bytes(saida)

    def _hex_string(self) -> bytes:
        self.pos += 1
        fim = self.buf.find(b">", self.pos)
        if fim < 0:
            fim = len(self.buf)
        corpo = re.sub(rb"[^0-9A-Fa-f]", b"", self.buf[self.pos : fim])
        self.pos = fim + 1
        if len(corpo) % 2:
            corpo += b"0"
        return binascii.unhexlify(corpo)

    def _dict(self):
        self.pos += 2
        dic = {}
        buf, n = self.buf, len(self.buf)
        while True:
            self.skip_ws()
            if self.pos >= n:
                break
            if buf[self.pos : self.pos + 2] == b">>":
                self.pos += 2
                break
            if buf[self.pos] != 0x2F:
                antes = self.pos
                self.parse()
                if self.pos == antes:
                    self.pos += 1
                continue
            chave = self._name()
            valor = self.parse()
            dic[str(chave)] = valor
        # stream?
        salvo = self.pos
        if self.peek_keyword() == b"stream":
            self.skip_ws()
            self.pos += len(b"stream")
            if buf[self.pos : self.pos + 2] == b"\r\n":
                self.pos += 2
            elif buf[self.pos : self.pos + 1] in (b"\n", b"\r"):
                self.pos += 1
            ini = self.pos
            comprimento = dic.get("Length")
            fim = -1
            if isinstance(comprimento, int) and comprimento >= 0:
                candidato = ini + comprimento
                trecho = buf[candidato : candidato + 20]
                if re.match(rb"\s*endstream", trecho):
                    fim = candidato
            if fim < 0:
                achado = buf.find(b"endstream", ini)
                fim = achado if achado >= 0 else n
                while fim > ini and buf[fim - 1] in b"\r\n":
                    fim -= 1
            bruto = buf[ini:fim]
            proximo = buf.find(b"endstream", fim)
            self.pos = (proximo + len(b"endstream")) if proximo >= 0 else n
            return Stream(dic, bruto)
        self.pos = salvo
        return dic


# --------------------------------------------------------------------------
# Filtros de stream
# --------------------------------------------------------------------------
def _ascii85(dados: bytes) -> bytes:
    dados = re.sub(rb"\s", b"", dados)
    if dados.startswith(b"<~"):
        dados = dados[2:]
    fim = dados.find(b"~>")
    if fim >= 0:
        dados = dados[:fim]
    try:
        return __import__("base64").a85decode(dados)
    except Exception:
        return b""


def _lzw(dados: bytes, early: int = 1) -> bytes:
    saida = bytearray()
    tabela = {i: bytes([i]) for i in range(256)}
    proximo, largura = 258, 9
    anterior = None
    buffer_bits = valor_bits = 0
    for byte in dados:
        buffer_bits = (buffer_bits << 8) | byte
        valor_bits += 8
        while valor_bits >= largura:
            codigo = (buffer_bits >> (valor_bits - largura)) & ((1 << largura) - 1)
            valor_bits -= largura
            if codigo == 256:
                tabela = {i: bytes([i]) for i in range(256)}
                proximo, largura, anterior = 258, 9, None
                continue
            if codigo == 257:
                return bytes(saida)
            if anterior is None:
                entrada = tabela.get(codigo, b"")
            elif codigo in tabela:
                entrada = tabela[codigo]
            else:
                entrada = anterior + anterior[:1]
            saida += entrada
            if anterior is not None:
                tabela[proximo] = anterior + entrada[:1]
                proximo += 1
                if proximo + early >= (1 << largura) and largura < 12:
                    largura += 1
            anterior = entrada
    return bytes(saida)


def _runlength(dados: bytes) -> bytes:
    saida = bytearray()
    i = 0
    while i < len(dados):
        n = dados[i]
        i += 1
        if n == 128:
            break
        if n < 128:
            saida += dados[i : i + n + 1]
            i += n + 1
        else:
            if i < len(dados):
                saida += bytes([dados[i]]) * (257 - n)
                i += 1
    return bytes(saida)


def _aplicar_preditor(dados: bytes, parms: dict) -> bytes:
    preditor = int(parms.get("Predictor", 1) or 1)
    if preditor <= 1:
        return dados
    colunas = int(parms.get("Columns", 1) or 1)
    cores = int(parms.get("Colors", 1) or 1)
    bpc = int(parms.get("BitsPerComponent", 8) or 8)
    bpp = max(1, (cores * bpc) // 8)
    linha_bytes = (colunas * cores * bpc + 7) // 8
    if preditor == 2:
        return dados
    saida = bytearray()
    anterior = bytearray(linha_bytes)
    passo = linha_bytes + 1
    for inicio in range(0, len(dados) - 1 + 1, passo):
        bloco = dados[inicio : inicio + passo]
        if len(bloco) < 2:
            break
        tipo = bloco[0]
        atual = bytearray(bloco[1:].ljust(linha_bytes, b"\x00"))
        for i in range(linha_bytes):
            a = atual[i - bpp] if i >= bpp else 0
            b = anterior[i]
            c = anterior[i - bpp] if i >= bpp else 0
            if tipo == 0:
                pass
            elif tipo == 1:
                atual[i] = (atual[i] + a) & 0xFF
            elif tipo == 2:
                atual[i] = (atual[i] + b) & 0xFF
            elif tipo == 3:
                atual[i] = (atual[i] + ((a + b) >> 1)) & 0xFF
            elif tipo == 4:
                p = a + b - c
                pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
                pr = a if (pa <= pb and pa <= pc) else (b if pb <= pc else c)
                atual[i] = (atual[i] + pr) & 0xFF
        saida += atual
        anterior = atual
    return bytes(saida)


def decodificar_stream(stream: Stream, doc: "Documento") -> bytes:
    dados = stream.raw
    filtros = doc.resolver(stream.dic.get("Filter"))
    if filtros is None:
        return dados
    if not isinstance(filtros, list):
        filtros = [filtros]
    parms = doc.resolver(stream.dic.get("DecodeParms")) or doc.resolver(stream.dic.get("DP"))
    if not isinstance(parms, list):
        parms = [parms]
    for i, filtro in enumerate(filtros):
        filtro = str(doc.resolver(filtro) or "")
        parm = doc.resolver(parms[i]) if i < len(parms) else None
        parm = parm if isinstance(parm, dict) else {}
        parm = {k: doc.resolver(v) for k, v in parm.items()}
        try:
            if filtro in ("FlateDecode", "Fl"):
                try:
                    dados = zlib.decompress(dados)
                except zlib.error:
                    obj = zlib.decompressobj()
                    dados = obj.decompress(dados)
                dados = _aplicar_preditor(dados, parm)
            elif filtro in ("LZWDecode", "LZW"):
                dados = _lzw(dados, int(parm.get("EarlyChange", 1) or 1))
                dados = _aplicar_preditor(dados, parm)
            elif filtro in ("ASCIIHexDecode", "AHx"):
                corpo = re.sub(rb"[^0-9A-Fa-f]", b"", dados.split(b">")[0])
                if len(corpo) % 2:
                    corpo += b"0"
                dados = binascii.unhexlify(corpo)
            elif filtro in ("ASCII85Decode", "A85"):
                dados = _ascii85(dados)
            elif filtro in ("RunLengthDecode", "RL"):
                dados = _runlength(dados)
            else:  # DCTDecode, JPXDecode, CCITTFaxDecode: imagem, nao e texto
                return b""
        except Exception:
            return b""
    return dados


# --------------------------------------------------------------------------
# Documento
# --------------------------------------------------------------------------
class Documento:
    def __init__(self, buf: bytes):
        self.buf = buf
        self.objetos: dict = {}
        self.avisos: list = []
        self._carregar_objetos()
        self._expandir_objstm()

    def _carregar_objetos(self) -> None:
        for m in re.finditer(rb"(?:^|[\s>\]])(\d{1,10})\s+(\d{1,5})\s+obj\b", self.buf):
            num = int(m.group(1))
            lexer = Lexer(self.buf, m.end())
            try:
                self.objetos[num] = lexer.parse()
            except Exception:
                continue

    def _expandir_objstm(self) -> None:
        for obj in list(self.objetos.values()):
            if not isinstance(obj, Stream):
                continue
            if str(self.resolver(obj.dic.get("Type")) or "") != "ObjStm":
                continue
            dados = decodificar_stream(obj, self)
            if not dados:
                continue
            n = int(self.resolver(obj.dic.get("N")) or 0)
            first = int(self.resolver(obj.dic.get("First")) or 0)
            cabecalho = dados[:first].split()
            for i in range(n):
                try:
                    num = int(cabecalho[2 * i])
                    desloc = int(cabecalho[2 * i + 1])
                except (IndexError, ValueError):
                    break
                if num in self.objetos:
                    continue
                try:
                    self.objetos[num] = Lexer(dados, first + desloc).parse()
                except Exception:
                    continue

    def resolver(self, obj, profundidade: int = 0):
        while isinstance(obj, Ref) and profundidade < 32:
            obj = self.objetos.get(obj.num)
            profundidade += 1
        return obj

    def dic_de(self, obj) -> dict:
        obj = self.resolver(obj)
        if isinstance(obj, Stream):
            return obj.dic
        return obj if isinstance(obj, dict) else {}

    def paginas(self) -> list:
        raiz = None
        for obj in self.objetos.values():
            dic = obj.dic if isinstance(obj, Stream) else obj
            if isinstance(dic, dict) and str(self.resolver(dic.get("Type")) or "") == "Catalog":
                raiz = dic
                break
        encontradas: list = []
        if raiz is not None:
            self._caminhar(self.resolver(raiz.get("Pages")), encontradas, set(), {})
        if not encontradas:
            self.avisos.append("Arvore de paginas nao encontrada: usando ordem dos objetos /Type /Page.")
            for num in sorted(self.objetos):
                dic = self.objetos[num]
                dic = dic.dic if isinstance(dic, Stream) else dic
                if isinstance(dic, dict) and str(self.resolver(dic.get("Type")) or "") == "Page":
                    encontradas.append(dic)
        return encontradas

    def _caminhar(self, no, saida: list, vistos: set, herdado: dict) -> None:
        no = self.resolver(no)
        if not isinstance(no, dict) or id(no) in vistos or len(saida) > 5000:
            return
        vistos.add(id(no))
        herdado = dict(herdado)
        for chave in ("Resources", "MediaBox", "CropBox", "Rotate"):
            if chave in no:
                herdado[chave] = no[chave]
        tipo = str(self.resolver(no.get("Type")) or "")
        filhos = self.resolver(no.get("Kids"))
        if tipo == "Page" or (filhos is None and "Contents" in no):
            pagina = dict(no)
            for chave, valor in herdado.items():
                pagina.setdefault(chave, valor)
            saida.append(pagina)
            return
        if isinstance(filhos, list):
            for filho in filhos:
                self._caminhar(filho, saida, vistos, herdado)


# --------------------------------------------------------------------------
# Fontes: bytes -> texto
# --------------------------------------------------------------------------
class Fonte:
    def __init__(self, doc: Documento, dic: dict):
        self.doc = doc
        self.dic = dic
        self.dois_bytes = False
        self.to_unicode: dict = {}
        self.larguras: dict = {}
        self.largura_padrao = 500.0
        self._carregar()

    def _carregar(self) -> None:
        doc, dic = self.doc, self.dic
        subtipo = str(doc.resolver(dic.get("Subtype")) or "")
        codificacao = doc.resolver(dic.get("Encoding"))
        nome_cod = str(codificacao) if isinstance(codificacao, Name) else ""
        if subtipo == "Type0" or nome_cod.startswith("Identity") or nome_cod.endswith("-H") or nome_cod.endswith("-V"):
            self.dois_bytes = True
        tu = doc.resolver(dic.get("ToUnicode"))
        if isinstance(tu, Stream):
            self._ler_cmap(decodificar_stream(tu, doc))
        if subtipo == "Type0":
            descendentes = doc.resolver(dic.get("DescendantFonts")) or []
            filho = doc.dic_de(descendentes[0]) if descendentes else {}
            self.largura_padrao = float(doc.resolver(filho.get("DW")) or 1000.0)
            self._ler_w(doc.resolver(filho.get("W")) or [])
        else:
            primeiro = doc.resolver(dic.get("FirstChar"))
            larguras = doc.resolver(dic.get("Widths"))
            if isinstance(larguras, list) and isinstance(primeiro, (int, float)):
                for i, largura in enumerate(larguras):
                    largura = doc.resolver(largura)
                    if isinstance(largura, (int, float)):
                        self.larguras[int(primeiro) + i] = float(largura)

    def _ler_w(self, w: list) -> None:
        w = [self.doc.resolver(x) for x in w]
        i = 0
        while i < len(w):
            if i + 1 < len(w) and isinstance(w[i + 1], list):
                inicio = int(w[i])
                for j, largura in enumerate(w[i + 1]):
                    largura = self.doc.resolver(largura)
                    if isinstance(largura, (int, float)):
                        self.larguras[inicio + j] = float(largura)
                i += 2
            elif i + 2 < len(w):
                try:
                    for codigo in range(int(w[i]), int(w[i + 1]) + 1):
                        self.larguras[codigo] = float(w[i + 2])
                except (TypeError, ValueError):
                    pass
                i += 3
            else:
                break

    def _ler_cmap(self, dados: bytes) -> None:
        def para_texto(bruto: bytes) -> str:
            try:
                return bruto.decode("utf-16-be", errors="ignore")
            except Exception:
                return ""

        for bloco in re.findall(rb"beginbfchar(.*?)endbfchar", dados, re.S):
            for origem, destino in re.findall(rb"<([0-9A-Fa-f]+)>\s*<([0-9A-Fa-f]*)>", bloco):
                self.to_unicode[int(origem, 16)] = para_texto(binascii.unhexlify(destino if len(destino) % 2 == 0 else destino + b"0"))
        for bloco in re.findall(rb"beginbfrange(.*?)endbfrange", dados, re.S):
            for m in re.finditer(rb"<([0-9A-Fa-f]+)>\s*<([0-9A-Fa-f]+)>\s*(?:<([0-9A-Fa-f]*)>|\[(.*?)\])", bloco, re.S):
                ini, fim = int(m.group(1), 16), int(m.group(2), 16)
                if fim - ini > 65535:
                    continue
                if m.group(3) is not None:
                    base = m.group(3)
                    base_texto = para_texto(binascii.unhexlify(base if len(base) % 2 == 0 else base + b"0"))
                    for k in range(fim - ini + 1):
                        if base_texto:
                            self.to_unicode[ini + k] = base_texto[:-1] + chr(ord(base_texto[-1]) + k)
                else:
                    itens = re.findall(rb"<([0-9A-Fa-f]*)>", m.group(4) or b"")
                    for k, item in enumerate(itens):
                        self.to_unicode[ini + k] = para_texto(binascii.unhexlify(item if len(item) % 2 == 0 else item + b"0"))

    def codigos(self, bruto: bytes):
        if self.dois_bytes:
            for i in range(0, len(bruto) - 1, 2):
                yield (bruto[i] << 8) | bruto[i + 1]
        else:
            for byte in bruto:
                yield byte

    def texto(self, codigo: int) -> str:
        if codigo in self.to_unicode:
            return self.to_unicode[codigo]
        if self.dois_bytes:
            return "�"
        return bytes([codigo]).decode("cp1252", errors="replace")

    def largura(self, codigo: int) -> float:
        return self.larguras.get(codigo, self.largura_padrao)


# --------------------------------------------------------------------------
# Extracao de texto posicionado
# --------------------------------------------------------------------------
def _mul(a, b):
    return (
        a[0] * b[0] + a[1] * b[2],
        a[0] * b[1] + a[1] * b[3],
        a[2] * b[0] + a[3] * b[2],
        a[2] * b[1] + a[3] * b[3],
        a[4] * b[0] + a[5] * b[2] + b[4],
        a[4] * b[1] + a[5] * b[3] + b[5],
    )


def _tokens(dados: bytes):
    lexer = Lexer(dados)
    operandos = []
    n = len(dados)
    while lexer.pos < n:
        lexer.skip_ws()
        if lexer.pos >= n:
            break
        c = dados[lexer.pos]
        if c in b"/([<+-." or 0x30 <= c <= 0x39:
            antes = lexer.pos
            operandos.append(lexer.parse())
            if lexer.pos == antes:
                lexer.pos += 1
            continue
        if c == 0x5B:
            operandos.append(lexer.parse())
            continue
        m = re.compile(rb"[A-Za-z'\"*01]+").match(dados, lexer.pos)
        if not m:
            lexer.pos += 1
            continue
        operador = m.group(0)
        lexer.pos = m.end()
        if operador == b"BI":  # imagem embutida: pula ate EI
            fim = dados.find(b"EI", lexer.pos)
            lexer.pos = (fim + 2) if fim >= 0 else n
            operandos = []
            continue
        if operador in (b"true", b"false", b"null"):
            operandos.append(operador == b"true")
            continue
        yield operador, operandos
        operandos = []


def extrair_itens(doc: Documento, pagina: dict) -> list:
    """Devolve itens de texto com posicao: [(x, y, tamanho, texto)]."""
    recursos = doc.dic_de(pagina.get("Resources"))
    dic_fontes = doc.dic_de(recursos.get("Font"))
    cache: dict = {}

    def pegar_fonte(nome: str):
        if nome not in cache:
            cache[nome] = Fonte(doc, doc.dic_de(dic_fontes.get(nome)))
        return cache[nome]

    conteudo = doc.resolver(pagina.get("Contents"))
    partes = []
    for item in conteudo if isinstance(conteudo, list) else [conteudo]:
        item = doc.resolver(item)
        if isinstance(item, Stream):
            partes.append(decodificar_stream(item, doc))
    dados = b"\n".join(p for p in partes if p)
    if not dados:
        return []

    itens: list = []
    ctm = (1.0, 0.0, 0.0, 1.0, 0.0, 0.0)
    pilha: list = []
    tm = tlm = (1.0, 0.0, 0.0, 1.0, 0.0, 0.0)
    fonte = None
    tamanho = 12.0
    tc = tw = ts = 0.0
    th = 1.0
    tl = 0.0

    def num(v, padrao=0.0):
        return float(v) if isinstance(v, (int, float)) else padrao

    def mostrar(bruto: bytes):
        nonlocal tm
        if fonte is None or not isinstance(bruto, bytes):
            return
        for codigo in fonte.codigos(bruto):
            glifo = fonte.texto(codigo)
            trm = _mul((tamanho * th, 0.0, 0.0, tamanho, 0.0, ts), _mul(tm, ctm))
            escala = abs(trm[3]) or abs(tamanho) or 1.0
            if glifo and glifo != "\x00":
                itens.append([trm[4], trm[5], escala, glifo])
            largura = fonte.largura(codigo) / 1000.0 * tamanho + tc
            if codigo == 32 and not fonte.dois_bytes:
                largura += tw
            tm = _mul((1.0, 0.0, 0.0, 1.0, largura * th, 0.0), tm)

    for operador, ops in _tokens(dados):
        try:
            if operador == b"q":
                pilha.append(ctm)
            elif operador == b"Q":
                if pilha:
                    ctm = pilha.pop()
            elif operador == b"cm" and len(ops) >= 6:
                ctm = _mul(tuple(num(v) for v in ops[-6:]), ctm)
            elif operador == b"BT":
                tm = tlm = (1.0, 0.0, 0.0, 1.0, 0.0, 0.0)
            elif operador == b"Tf" and len(ops) >= 2:
                fonte = pegar_fonte(str(ops[-2]))
                tamanho = num(ops[-1], 12.0)
            elif operador == b"Td" and len(ops) >= 2:
                tm = tlm = _mul((1.0, 0.0, 0.0, 1.0, num(ops[-2]), num(ops[-1])), tlm)
            elif operador == b"TD" and len(ops) >= 2:
                tl = -num(ops[-1])
                tm = tlm = _mul((1.0, 0.0, 0.0, 1.0, num(ops[-2]), num(ops[-1])), tlm)
            elif operador == b"Tm" and len(ops) >= 6:
                tm = tlm = tuple(num(v) for v in ops[-6:])
            elif operador == b"T*":
                tm = tlm = _mul((1.0, 0.0, 0.0, 1.0, 0.0, -tl), tlm)
            elif operador == b"TL" and ops:
                tl = num(ops[-1])
            elif operador == b"Tc" and ops:
                tc = num(ops[-1])
            elif operador == b"Tw" and ops:
                tw = num(ops[-1])
            elif operador == b"Tz" and ops:
                th = num(ops[-1], 100.0) / 100.0
            elif operador == b"Ts" and ops:
                ts = num(ops[-1])
            elif operador == b"Tj" and ops:
                mostrar(ops[-1])
            elif operador == b"'" and ops:
                tm = tlm = _mul((1.0, 0.0, 0.0, 1.0, 0.0, -tl), tlm)
                mostrar(ops[-1])
            elif operador == b'"' and len(ops) >= 3:
                tw, tc = num(ops[-3]), num(ops[-2])
                tm = tlm = _mul((1.0, 0.0, 0.0, 1.0, 0.0, -tl), tlm)
                mostrar(ops[-1])
            elif operador == b"TJ" and ops and isinstance(ops[-1], list):
                for elemento in ops[-1]:
                    if isinstance(elemento, bytes):
                        mostrar(elemento)
                    elif isinstance(elemento, (int, float)):
                        ajuste = -float(elemento) / 1000.0 * tamanho * th
                        tm = _mul((1.0, 0.0, 0.0, 1.0, ajuste, 0.0), tm)
        except Exception:
            continue
    return itens


def montar_linhas(itens: list) -> list:
    """Agrupa itens posicionados em linhas de texto, preservando colunas."""
    if not itens:
        return []
    tamanho_medio = sorted(i[2] for i in itens)[len(itens) // 2] or 10.0
    tolerancia = max(1.0, tamanho_medio * 0.4)
    itens = sorted(itens, key=lambda i: (-i[1], i[0]))
    linhas: list = []
    atual: list = [itens[0]]
    for item in itens[1:]:
        if abs(item[1] - atual[-1][1]) <= tolerancia:
            atual.append(item)
        else:
            linhas.append(atual)
            atual = [item]
    linhas.append(atual)

    saida = []
    for linha in linhas:
        linha.sort(key=lambda i: i[0])
        texto = ""
        fim_anterior = None
        for x, _y, tam, glifo in linha:
            largura_car = max(tam * 0.45, 1.0)
            if fim_anterior is not None:
                lacuna = x - fim_anterior
                if lacuna > largura_car * 2.5:
                    texto += " " * min(int(lacuna / largura_car), 12)
                elif lacuna > largura_car * 0.28 and not texto.endswith(" "):
                    texto += " "
            texto += glifo
            fim_anterior = x + tam * 0.5 * max(len(glifo), 1)
        texto = texto.rstrip()
        if texto.strip():
            saida.append(texto)
    return saida


# --------------------------------------------------------------------------
# Motores de extracao
# --------------------------------------------------------------------------
def motor_interno(caminho: Path) -> tuple:
    doc = Documento(caminho.read_bytes())
    paginas = doc.paginas()
    resultado = []
    for pagina in paginas:
        resultado.append(montar_linhas(extrair_itens(doc, pagina)))
    return resultado, doc.avisos


def motor_pdftotext(caminho: Path) -> tuple:
    saida = subprocess.run(
        ["pdftotext", "-layout", "-enc", "UTF-8", str(caminho), "-"],
        capture_output=True,
        timeout=180,
    )
    if saida.returncode != 0:
        raise RuntimeError(saida.stderr.decode("utf-8", "replace").strip() or "pdftotext falhou")
    texto = saida.stdout.decode("utf-8", "replace")
    paginas = texto.split("\f")
    if paginas and not paginas[-1].strip():
        paginas.pop()
    return [[linha.rstrip() for linha in p.split("\n") if linha.strip()] for p in paginas], []


# --------------------------------------------------------------------------
# Estruturacao (heuristicas explicitas, sem interpretacao semantica)
# --------------------------------------------------------------------------
RE_CAMPO = re.compile(r"^\s*([^:=]{2,60}?)\s*[:=]\s*(\S.*)$")
RE_COLUNAS = re.compile(r"^\s*(\S.{0,58}?)\s{2,}(\S.*)$")
RE_TITULO_NUM = re.compile(r"^\s*(\d+(?:\.\d+)*)[.)]?\s+(\S.{1,80})$")
RE_LISTA = re.compile(r"^\s*(?:[-*•●·>]|\(?[a-z]\)|\d+[.)])\s+(\S.*)$")
RE_UNIDADE = re.compile(
    r"(?<![A-Za-z0-9])("
    r"\d+(?:[.,]\d+)?\s?(?:mm|cm|m|km|kg|g|mg|ml|l|in|pol|pt|px|dpi|ppi|%|°C?|un|pcs)"
    r"|#[0-9A-Fa-f]{6}|#[0-9A-Fa-f]{3}"
    r"|RGB\s*\(?\s*\d{1,3}\s*,\s*\d{1,3}\s*,\s*\d{1,3}"
    r"|CMYK\s*\(?\s*\d{1,3}"
    r"|Pantone\s*[0-9A-Za-z\-]+"
    r")",
    re.IGNORECASE,
)
PALAVRAS_VISUAIS = (
    "cor", "cores", "acabamento", "material", "materiais", "dimens", "medida", "altura",
    "largura", "profundidade", "comprimento", "diametro", "diâmetro", "peso", "textura",
    "formato", "forma", "espessura", "raio", "angulo", "ângulo", "revestimento", "pintura",
    "logotipo", "logo", "tipografia", "fonte", "layout", "posicao", "posição", "borda",
    "cantos", "superficie", "superfície", "embalagem", "rotulo", "rótulo",
)


def dividir_campos(linha: str) -> list:
    """Separa uma linha com varios rotulos ("A: 1    B: 2") em pares chave/valor.

    So divide quando ha pelo menos dois segmentos com rotulo, para nao quebrar
    valores que contenham dois-pontos no meio do texto.
    """
    segmentos = [s for s in re.split(r"\s{2,}", linha.strip()) if s]
    if len(segmentos) < 2:
        return []
    if sum(1 for s in segmentos if RE_CAMPO.match(s)) < 2:
        return []
    pares, atual = [], None
    for segmento in segmentos:
        m = RE_CAMPO.match(segmento)
        if m:
            atual = [m.group(1).strip(), m.group(2).strip()]
            pares.append(atual)
        elif atual is not None:
            atual[1] = f"{atual[1]} {segmento.strip()}".strip()
    return [(chave, valor) for chave, valor in pares if chave and valor]


def normalizar(texto: str) -> str:
    texto = unicodedata.normalize("NFKD", texto)
    texto = "".join(c for c in texto if not unicodedata.combining(c))
    return re.sub(r"\s+", " ", texto).strip().lower()


def estruturar(paginas: list) -> dict:
    campos, titulos, listas, visuais = [], [], [], []
    vistos_campos = set()
    for numero, linhas in enumerate(paginas, start=1):
        for indice, linha in enumerate(linhas, start=1):
            fonte = {"pagina": numero, "linha": indice}
            enxuta = linha.strip()
            m = RE_TITULO_NUM.match(linha)
            e_titulo = False
            if m and len(enxuta) <= 90 and not RE_CAMPO.match(linha):
                titulos.append({"texto": enxuta, "numeracao": m.group(1), **fonte})
                e_titulo = True
            elif enxuta == enxuta.upper() and 3 <= len(enxuta) <= 70 and re.search(r"[A-ZÀ-Ý]", linha):
                titulos.append({"texto": enxuta, "numeracao": None, **fonte})
                e_titulo = True

            m = RE_LISTA.match(linha)
            if m and not e_titulo:
                listas.append({"texto": m.group(1).strip(), **fonte})

            achados = []
            for chave_div, valor_div in dividir_campos(linha):
                achados.append((chave_div, valor_div, "rotulo"))
            if not achados:
                m = RE_CAMPO.match(linha)
                if m and m.group(1).strip() and m.group(2).strip():
                    achados.append((m.group(1).strip(), m.group(2).strip(), "rotulo"))
                else:
                    m = RE_COLUNAS.match(linha)
                    if m and not RE_LISTA.match(linha) and not e_titulo:
                        achados.append((m.group(1).strip(), re.sub(r"\s{2,}", " | ", m.group(2).strip()), "tabela"))
            for chave_ach, valor_ach, origem_ach in achados:
                assinatura = (normalizar(chave_ach), normalizar(valor_ach))
                if assinatura in vistos_campos:
                    continue
                vistos_campos.add(assinatura)
                campos.append({"chave": chave_ach, "valor": valor_ach, "origem": origem_ach, **fonte})

            chave = achados[0][0] if achados else None
            valor = achados[0][1] if achados else None
            unidades = sorted({u.group(1) for u in RE_UNIDADE.finditer(linha)})
            chave_norm = normalizar(chave or linha)
            if unidades or any(p in chave_norm for p in PALAVRAS_VISUAIS):
                visuais.append(
                    {
                        "texto": linha.strip(),
                        "chave": chave,
                        "valor": valor,
                        "unidades": unidades,
                        **fonte,
                    }
                )
    return {"titulos": titulos, "campos": campos, "listas": listas, "atributos_visuais": visuais}


# --------------------------------------------------------------------------
# Escrita das saidas
# --------------------------------------------------------------------------
def slugificar(nome: str) -> str:
    base = normalizar(nome)
    base = re.sub(r"[^a-z0-9]+", "-", base).strip("-")
    return base or "documento"


def escrever_md(spec: dict) -> str:
    doc = spec["documento"]
    linhas = [
        f"# Extracao: {doc['arquivo']}",
        "",
        "> Conteudo extraido automaticamente do PDF. Nada aqui foi interpretado,",
        "> resumido ou reescrito. Cada item indica pagina e linha de origem.",
        "",
        "| Metadado | Valor |",
        "| --- | --- |",
        f"| Arquivo | `{doc['arquivo']}` |",
        f"| SHA-256 | `{doc['sha256']}` |",
        f"| Paginas | {doc['paginas']} |",
        f"| Motor | {doc['motor']} |",
        f"| Extraido em | {doc['extraido_em']} |",
        f"| Extrator | `extrair_spec_pdf.py` v{doc['versao_extrator']} |",
        "",
    ]
    if spec["documento"].get("avisos"):
        linhas += ["## Avisos", ""] + [f"- {a}" for a in spec["documento"]["avisos"]] + [""]

    linhas += ["## Campos identificados", "", "| Chave | Valor | Origem | Pagina | Linha |", "| --- | --- | --- | --- | --- |"]
    for campo in spec["campos"]:
        chave = campo["chave"].replace("|", "\\|")
        valor = campo["valor"].replace("|", "\\|")
        linhas.append(f"| {chave} | {valor} | {campo['origem']} | {campo['pagina']} | {campo['linha']} |")
    if not spec["campos"]:
        linhas.append("| _nenhum_ | pendente-validacao | - | - | - |")

    linhas += ["", "## Atributos com medida, cor ou material", ""]
    for item in spec["atributos_visuais"]:
        linhas.append(f"- {item['texto']}  _(p. {item['pagina']}, l. {item['linha']})_")
    if not spec["atributos_visuais"]:
        linhas.append("- pendente-validacao: nenhuma medida, cor ou material reconhecido no texto extraido.")

    linhas += ["", "## Titulos", ""]
    for titulo in spec["titulos"]:
        linhas.append(f"- {titulo['texto']}  _(p. {titulo['pagina']}, l. {titulo['linha']})_")
    if not spec["titulos"]:
        linhas.append("- pendente-validacao")
    linhas.append("")
    return "\n".join(linhas)


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(
        description="Extrai, sem interpretar, o conteudo de um PDF de especificacoes.",
    )
    parser.add_argument("pdf", help="caminho do PDF de especificacoes")
    parser.add_argument("-o", "--saida", default="knowledge/extracted", help="diretorio base de saida (padrao: knowledge/extracted)")
    parser.add_argument("-n", "--nome", help="nome da pasta de saida (padrao: slug do arquivo)")
    parser.add_argument("-m", "--motor", choices=["auto", "interno", "pdftotext"], default="auto", help="motor de extracao (padrao: auto)")
    parser.add_argument("--preservar", metavar="DIR", nargs="?", const="knowledge/raw", help="copia o PDF original para DIR (padrao: knowledge/raw), sem sobrescrever")
    parser.add_argument("--min-chars", type=int, default=40, help="minimo de caracteres para considerar que o PDF tem camada de texto (padrao: 40)")
    parser.add_argument("--forcar", action="store_true", help="sobrescreve a pasta de saida se ja existir")
    args = parser.parse_args(argv)

    caminho = Path(args.pdf).expanduser()
    if not caminho.is_file():
        print(f"[ERRO] arquivo nao encontrado: {caminho}", file=sys.stderr)
        return 1

    bruto = caminho.read_bytes()
    if not bruto.startswith(b"%PDF"):
        print(f"[AVISO] {caminho.name} nao comeca com %PDF; tentando mesmo assim.", file=sys.stderr)

    motor = args.motor
    avisos: list = []
    if motor == "auto":
        motor = "pdftotext" if shutil.which("pdftotext") else "interno"
    try:
        if motor == "pdftotext":
            paginas, avisos = motor_pdftotext(caminho)
        else:
            paginas, avisos = motor_interno(caminho)
    except Exception as erro:
        if motor == "pdftotext":
            print(f"[AVISO] pdftotext falhou ({erro}); usando o motor interno.", file=sys.stderr)
            motor = "interno"
            paginas, avisos = motor_interno(caminho)
        else:
            print(f"[ERRO] falha ao extrair: {erro}", file=sys.stderr)
            return 1

    total_chars = sum(len(l) for p in paginas for l in p)
    if total_chars < args.min_chars:
        print(
            "[ERRO] o PDF nao tem camada de texto extraivel "
            f"({total_chars} caracteres em {len(paginas)} pagina(s)).\n"
            "       Rode OCR antes (ex.: ocrmypdf entrada.pdf saida.pdf) e extraia de novo.\n"
            "       Nao substitua a extracao por leitura visual do documento.",
            file=sys.stderr,
        )
        return 2

    destino = Path(args.saida) / (args.nome or slugificar(caminho.stem))
    if destino.exists() and any(destino.iterdir()) and not args.forcar:
        print(f"[ERRO] {destino} ja existe e nao esta vazia. Use --forcar para sobrescrever.", file=sys.stderr)
        return 1
    destino.mkdir(parents=True, exist_ok=True)

    if args.preservar:
        pasta_raw = Path(args.preservar)
        pasta_raw.mkdir(parents=True, exist_ok=True)
        alvo = pasta_raw / caminho.name
        if alvo.resolve() != caminho.resolve():
            if alvo.exists():
                print(f"[AVISO] original ja preservado em {alvo}; nada copiado.", file=sys.stderr)
            else:
                shutil.copy2(caminho, alvo)

    spec = {
        "documento": {
            "arquivo": caminho.name,
            "caminho_original": str(caminho),
            "sha256": hashlib.sha256(bruto).hexdigest(),
            "bytes": len(bruto),
            "paginas": len(paginas),
            "motor": motor,
            "caracteres": total_chars,
            "extraido_em": datetime.now(timezone.utc).astimezone().isoformat(timespec="seconds"),
            "versao_extrator": VERSAO,
            "avisos": avisos,
        },
        "paginas": [
            {"numero": n, "linhas": [{"linha": i, "texto": t} for i, t in enumerate(linhas, start=1)]}
            for n, linhas in enumerate(paginas, start=1)
        ],
    }
    spec.update(estruturar(paginas))

    texto = "\n".join(
        f"=== pagina {n} ===\n" + "\n".join(linhas) for n, linhas in enumerate(paginas, start=1)
    )
    (destino / "texto.txt").write_text(texto + "\n", encoding="utf-8")
    (destino / "spec.json").write_text(json.dumps(spec, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    (destino / "spec.md").write_text(escrever_md(spec), encoding="utf-8")

    print(f"[  OK] motor            : {motor}")
    print(f"[  OK] paginas          : {len(paginas)}")
    print(f"[  OK] caracteres       : {total_chars}")
    print(f"[  OK] campos           : {len(spec['campos'])}")
    print(f"[  OK] atributos visuais: {len(spec['atributos_visuais'])}")
    for aviso in avisos:
        print(f"[AVISO] {aviso}", file=sys.stderr)
    print(f"[  OK] saida            : {destino}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
