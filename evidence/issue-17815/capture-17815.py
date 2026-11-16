#!/usr/bin/env python3
"""Captura la evidencia del issue #17815 (PR #17862): el pivote del orbit.

El MISMO script y los MISMOS numeros corren contra el binario BEFORE y el
AFTER: lo unico que cambia es el binario que se lanzo. Por eso el script solo
dice QUE capturar y con que nombre.

Que muestra:
  1. una caja de 40x30x20 (la del test de regresion) y Home la encuadra;
  2. la rueda hace zoom al cursor, lejos del centro de la vista, y la caja sale
     del centro;
  3. un arrastre con el boton del medio la desplaza mas lejos todavia;
  4. un arrastre con Shift y el boton del medio orbita (Fusion: Shift+medio
     orbita, el medio sin Shift desplaza);
  y mide, con ui.worldToScreen, donde esta el centro de la caja en pantalla
  antes y despues del orbit.

  BEFORE (origin/main): la caja SE MUEVE de su sitio mientras la vista gira:
    orbita alrededor del punto de encuadre, en el que la caja ya no esta.
  AFTER (PR): la caja GIRA EN SU SITIO: se queda en los mismos pixeles.

Uso: capture-17815.py <puerto> <token> <directorio-de-salida> <prefijo>
"""
import json
import os
import socket
import sys
import time

HOST = "127.0.0.1"
PORT = int(sys.argv[1]) if len(sys.argv) > 1 else 27891
TOKEN = sys.argv[2] if len(sys.argv) > 2 else ""
OUTDIR = sys.argv[3] if len(sys.argv) > 3 else "/tmp/evidence-17815"
PREFIX = sys.argv[4] if len(sys.argv) > 4 else "capture"

# El tamano de la ventana: el de la imagen que se publica.
WIDTH, HEIGHT = 1280, 720

# La caja del test de regresion: solid.box 40x30x20 desde el origen.
BOX = {"length": 40, "width": 30, "height": 20}
# Su centro en el mundo (la caja va de (0,0,0) a (40,30,20)).
CENTRE = [20.0, 15.0, 10.0]

# Los numeros del gesto. Los del test de regresion son los que hay por defecto;
# las variables de entorno los cambian (el MISMO juego en los dos binarios).
SCROLL_DELTA = float(os.environ.get("SCROLL_DELTA", "120"))
SCROLL_AT = (float(os.environ.get("SCROLL_AT_X", "500")), float(os.environ.get("SCROLL_AT_Y", "250")))
PAN_FROM = (float(os.environ.get("PAN_FROM_X", "300")), float(os.environ.get("PAN_FROM_Y", "300")))
PAN_DELTA = (float(os.environ.get("PAN_DX", "60")), float(os.environ.get("PAN_DY", "40")))
ORBIT_FROM = (float(os.environ.get("ORBIT_FROM_X", "300")), float(os.environ.get("ORBIT_FROM_Y", "300")))
ORBIT_DELTA = (float(os.environ.get("ORBIT_DX", "60")), float(os.environ.get("ORBIT_DY", "30")))


class Conn:
    def __init__(self, host, port, token):
        self.s = socket.create_connection((host, port), timeout=120)
        self.f = self.s.makefile("rwb")
        self.token = token
        self.authed = False
        self.id = 0

    def __call__(self, method, params=None, timeout=180):
        self.id += 1
        msg = {"id": self.id, "method": method, "params": params or {}}
        if not self.authed:
            msg["token"] = self.token
        self.f.write((json.dumps(msg) + "\n").encode())
        self.f.flush()
        self.s.settimeout(timeout)
        line = self.f.readline()
        if not line:
            raise RuntimeError(f"sin respuesta de {method}")
        self.authed = True
        reply = json.loads(line)
        if not reply.get("ok", False):
            print(f"  AVISO {method}: {json.dumps(reply)[:300]}")
        return reply

    def execute(self, command, params=None):
        return self("engine.execute", {"command": command, "params": params or {}})


def main():
    os.makedirs(OUTDIR, exist_ok=True)
    c = Conn(HOST, PORT, TOKEN)
    log = {"prefijo": PREFIX, "puerto": PORT, "pasos": []}

    def pump(k=8, each=0.06):
        """La app procesa UN evento de puntero por frame: hay que dejar correr
        el loop entre un evento y el siguiente."""
        for _ in range(k):
            time.sleep(each)
            c("ui.inspect", {})

    def inspect():
        return (c("ui.inspect", {}).get("result") or {})

    def rect():
        r = inspect().get("viewport")
        if not r:
            raise RuntimeError("la app no tiene viewport")
        left, top, w, h = (float(v) for v in r)
        return (left, top, w, h)

    def at(rel_x, rel_y):
        """Un punto de la vista, en coordenadas de la ventana."""
        left, top, w, h = rect()
        x = min(max(left + rel_x, left + 8.0), left + w - 8.0)
        y = min(max(top + rel_y, top + 8.0), top + h - 8.0)
        return (x, y)

    def centre_screen():
        pts = (c("ui.worldToScreen", {"points": [CENTRE]}).get("result") or [None])[0]
        return None if not pts else (float(pts[0]), float(pts[1]))

    def cam():
        return inspect().get("camera")

    def note(tag, extra=None):
        rec = {"tag": tag, "cam": cam()}
        if extra:
            rec.update(extra)
        log["pasos"].append(rec)
        return rec

    def shot(name):
        path = os.path.join(OUTDIR, f"{PREFIX}-{name}.png")
        r = c("ui.screenshot", {"path": path})
        ok = r.get("ok", False)
        print(f"  ui.screenshot {os.path.basename(path)} -> {'ok' if ok else json.dumps(r)[:200]}")
        log["pasos"].append({"tag": "screenshot", "name": os.path.basename(path), "ok": ok})
        return path if ok else None

    print("=== 0. la ventana y la escena ===")
    c("ui.resize", {"width": WIDTH, "height": HEIGHT})
    pump(8)
    if inspect().get("home"):
        print("  la pagina de inicio estaba abierta: se cierra")
        c("ui.home", {"open": False})
        pump(6)
    r = c.execute("solid.box", BOX)
    print("  solid.box ->", json.dumps(r.get("result") or r.get("error"))[:200])
    pump(10)
    r = c("ui.view", {"view": "home"})
    print("  ui.view home ->", json.dumps(r.get("result") or r.get("error"))[:200])
    pump(20)
    left, top, w, h = rect()
    print(f"  el viewport: left={left} top={top} w={w} h={h}")
    framed = centre_screen()
    print("  el centro de la caja tras el encuadre:", framed)

    print("\n=== 1. zoom al cursor, lejos del centro de la vista ===")
    sx, sy = at(*SCROLL_AT)
    print(f"  la rueda en ({sx}, {sy}), delta {SCROLL_DELTA}")
    c("ui.scroll", {"x": sx, "y": sy, "delta": SCROLL_DELTA})
    pump(20)
    after_zoom = centre_screen()
    print("  el centro de la caja tras el zoom:", after_zoom)

    print("\n=== 2. desplazamiento (boton del medio) ===")
    x0, y0 = at(*PAN_FROM)
    x1, y1 = at(PAN_FROM[0] + PAN_DELTA[0], PAN_FROM[1] + PAN_DELTA[1])
    print(f"  el arrastre de ({x0}, {y0}) a ({x1}, {y1})")
    c("ui.drag", {"x0": x0, "y0": y0, "x1": x1, "y1": y1, "button": "middle", "steps": 6})
    pump(20)
    before_orbit = centre_screen()
    print("  el centro de la caja antes del orbit:", before_orbit)
    note("antes-del-orbit", {"centro_caja": before_orbit})
    shot("zoom-pan")

    print("\n=== 3. orbit (Shift + boton del medio) ===")
    ox, oy = at(*ORBIT_FROM)
    ex, ey = at(ORBIT_FROM[0] + ORBIT_DELTA[0], ORBIT_FROM[1] + ORBIT_DELTA[1])
    print(f"  el arrastre de ({ox}, {oy}) a ({ex}, {ey})")
    c("ui.drag", {"x0": ox, "y0": oy, "x1": ex, "y1": ey, "button": "middle",
                  "shift": True, "steps": 10})
    pump(25)
    after_orbit = centre_screen()
    print("  el centro de la caja despues del orbit:", after_orbit)
    note("despues-del-orbit", {"centro_caja": after_orbit})
    shot("after-the-orbit")

    moved = None
    if before_orbit and after_orbit:
        moved = ((after_orbit[0] - before_orbit[0]) ** 2 + (after_orbit[1] - before_orbit[1]) ** 2) ** 0.5
        print(f"  el centro de la caja se movio {moved:.2f} pixeles")

    log.update({
        "viewport": [left, top, w, h],
        "centro_caja_encuadre": framed,
        "centro_caja_tras_el_zoom": after_zoom,
        "centro_caja_antes_del_orbit": before_orbit,
        "centro_caja_despues_del_orbit": after_orbit,
        "desplazamiento_pixeles": moved,
        "gesto": {
            "rueda": {"x": sx, "y": sy, "delta": SCROLL_DELTA},
            "desplazamiento": {"x0": x0, "y0": y0, "x1": x1, "y1": y1, "boton": "medio", "pasos": 6},
            "orbit": {"x0": ox, "y0": oy, "x1": ex, "y1": ey, "boton": "medio", "shift": True, "pasos": 10},
        },
        "tamano_imagen": [WIDTH, HEIGHT],
    })
    with open(os.path.join(OUTDIR, f"{PREFIX}-estado.json"), "w") as fh:
        json.dump(log, fh, indent=1)

    print("\n=== resumen ===")
    print("  el centro de la caja antes del orbit:", before_orbit)
    print("  el centro de la caja despues del orbit:", after_orbit)
    print(f"  se movio {moved:.2f} pixeles" if moved is not None else "  no se pudo medir")
    for f in sorted(os.listdir(OUTDIR)):
        if f.endswith(".png"):
            print("  ", f, os.path.getsize(os.path.join(OUTDIR, f)))
    return 0


if __name__ == "__main__":
    sys.exit(main())
