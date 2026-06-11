import marimo

__generated_with = "0.23.9"
app = marimo.App(width="medium", app_title="Python intro")


@app.cell(hide_code=True)
def _():
    import marimo as mo
    import numpy as np
    import io
    import contextlib
    import matplotlib.pyplot as plt
    import inspect

    def clean_code(source):
        return inspect.cleandoc(source)

    def render_task_output(mo, actual_output, target_output):
        actual = actual_output.rstrip()
        target = inspect.cleandoc(target_output).rstrip()
        actual_lines = actual.splitlines()
        target_lines = target.splitlines()

        exact_matches = {
            index
            for index, (actual_line, target_line) in enumerate(zip(actual_lines, target_lines))
            if actual_line == target_line
        }

        used_target_indices = set(exact_matches)
        actual_block = []

        for index, line in enumerate(actual_lines):
            if index in exact_matches:
                actual_block.append(f"✅ {line}")
                continue

            moved_target_index = next(
                (
                    target_index
                    for target_index, target_line in enumerate(target_lines)
                    if target_index not in used_target_indices and target_line == line
                ),
                None,
            )

            if moved_target_index is not None:
                used_target_indices.add(moved_target_index)
                actual_block.append(f"🟡 {line}")
            elif index < len(target_lines) and index not in used_target_indices:
                used_target_indices.add(index)
                actual_block.append(f"🔴 {line} → soll: {target_lines[index]}")
            else:
                actual_block.append(f"🔴 extra: {line}")

        missing_target_lines = [
            line for index, line in enumerate(target_lines) if index not in used_target_indices
        ]
        actual_block.extend(f"🔴 fehlt: {line}" for line in missing_target_lines)
        target_block = target_lines

        if not actual_block:
            actual_block = ["🔴 (keine Ausgabe)"]
        if not target_block:
            target_block = ["(keine Zielausgabe)"]

        actual_text = "\n".join(actual_block).replace("```", "''' ")
        target_text = "\n".join(target_block).replace("```", "''' ")

        return mo.md(
            "**Aktuelle Ausgabe**\n\n"
            f"``` text\n{actual_text}\n```\n\n"
            "**Zielausgabe**\n\n"
            f"``` text\n{target_text}\n```"
        )

    return clean_code, contextlib, inspect, io, mo, np, plt, render_task_output


@app.cell(hide_code=True)
def intro(mo):
    mo.md(rf"""
    # Python-Intro

    ## Was ist Python?

    Python ist eine Programmiersprache, die viele Anwendungsmöglichkeiten hat. Zum einen ist sie vergleichsweise einfach zu lesen und zu schreiben, allerdings bietet sie auch viele Möglichkeiten und ist deswegen auch für fortgeschrittene Programmierer interessant. Python wird zum Beispiel in der Webentwicklung, in der Datenanalyse, im maschinellen Lernen und in der Automatisierung eingesetzt.

    Python ist eine sogenannte "High-Level"-Programmiersprache, was bedeutet, dass sie sich mehr an der menschlichen Sprache orientiert als an der Maschinensprache. Allerdings steckt hinter der einfachen Syntax (Grammatik) eine Menge an Optimierung und Effizienz, so dass Python-Code oft sehr schnell ausgeführt werden kann.

    ## Hinweis

    Dieses Tutorial fängt mit den Grundlagen vom Programmieren an. Wer schon Erfahrung mit anderen Programmiersprachen hat,

    kann natürlich auch direkt zu den fortgeschrittenen Themen springen. Aber Achtung, die fortgeschrittenen Themen bauen auf

    den Grundlagen auf!

    ---
    """)
    return


@app.cell(hide_code=True)
def kapitel(mo):
    mo.md(rf"""
    ## Kapitel

    - **Syntax eines Python-Programms**
    - **Datentypen und Variablen**
    - **Operatoren und Ausdrücke**
    - **Kontrollstrukturen (if, for, while)**
    - **Funktionen**
    - **Erweiterte Datenstrukturen (Listen, Tupel)**
    - **Module und Pakete**
    - **NumPy-Basics**

    ---
    """)
    return


@app.cell(hide_code=True)
def _(mo):
    mo.md(r"""
    ### Hilfe zu eingebauten Funktionen und Klassen

    Gib den Namen einer eingebauten Python-Funktion oder -Klasse ein, z. B. `print`, `len`, `list`, `dict`, `range` oder `int`.

    Es wird **kein Code ausgeführt**. Der Name wird nur in Pythons eingebauten Objekten nachgeschlagen.
    """)
    return


@app.cell(hide_code=True)
def _(mo):
    builtin_help_query = mo.ui.text(
        label="Name",
        placeholder="z. B. len, print, list",
        value="len",
    )
    builtin_help_query
    return (builtin_help_query,)


@app.cell(hide_code=True)
def _(builtin_help_query, inspect, mo):
    import builtins

    query = builtin_help_query.value.strip()

    if not query:
        help_output = mo.md("Gib oben einen Namen ein, um die Hilfe anzuzeigen.")
    elif not query.isidentifier():
        help_output = mo.md(
            f"⚠️ `{query}` ist kein einzelner Python-Name.\n\n"
            "Bitte gib nur den Namen ein, z. B. `len` statt `len()` oder `math.sqrt`."
        )
    elif query.startswith("_"):
        help_output = mo.md("⚠️ Namen, die mit `_` beginnen, werden hier nicht angezeigt.")
    elif not hasattr(builtins, query):
        help_output = mo.md(
            f"⚠️ `{query}` wurde nicht in den eingebauten Python-Namen gefunden.\n\n"
            "Beispiele, die funktionieren: `len`, `print`, `list`, `dict`, `range`, `int`, `str`."
        )
    else:
        obj = getattr(builtins, query)
        if not (inspect.isclass(obj) or callable(obj)):
            help_output = mo.md(
                f"⚠️ `{query}` ist eingebaut, aber keine Funktion oder Klasse.\n\n"
                "Probiere z. B. `len`, `print`, `list`, `dict`, `range`, `int` oder `str`."
            )
        else:
            try:
                kind = "Klasse" if inspect.isclass(obj) else "Funktion"
                try:
                    call_text = f"{query}{inspect.signature(obj)}"
                except (TypeError, ValueError):
                    call_text = f"{query}(...)"

                help_text = inspect.getdoc(obj) or "Keine Beschreibung vorhanden."
                max_length = 3000
                if len(help_text) > max_length:
                    help_text = help_text[:max_length].rstrip() + "\n\n... gekürzt ..."

                help_text = help_text.replace("```", "'''")
                help_output = mo.md(
                    f"**Hilfe für `{query}`**\n\n"
                    f"- Typ: **{kind}**\n"
                    f"- Aufruf: `{call_text}`\n\n"
                    "``` text\n"
                    f"{help_text}\n"
                    "```"
                )
            except Exception as error:
                help_output = mo.md(
                    f"⚠️ Für `{query}` konnte keine Hilfe erzeugt werden.\n\n"
                    f"Fehler: `{error}`"
                )

    help_output
    return


@app.cell(hide_code=True)
def syntax(code_row_slider, mo):
    _programm = rf"""

    ## Syntax eines Python-Programms

    ### Reihenfolge

    Python-Programme werden von oben nach unten ausgeführt. Das bedeutet, dass der Code in der Reihenfolge ausgeführt wird, in der er geschrieben ist. Es gibt jedoch einige Ausnahmen, aber dazu kommen wir später.

    ```python

    # In Python werden Kommentare mit einem Hashtag (#) eingeleitet.

    # Diese beiden Zeilen werden also ignoriert

    """

    _programm += rf"""
    print("Hallo")
    print("Tschüss")
    ```

    Weil wir von oben nach unten ausführen, wird erst 'Hallo' gesagt, danach 'Tschüss'.

    """ if code_row_slider.value == 4 else rf"""
    print("Tschüss")
    print("Hallo")
    ```

    Weil wir von oben nach unten ausführen, wird erst 'Tschüss' gesagt, danach 'Hallo'.

    """

    mo.md(_programm)
    return


@app.cell(hide_code=True)
def _(mo):
    code_row_slider = mo.ui.slider(steps=[4, 5], label="\"Hallo\" in Zeile")
    code_row_slider
    return (code_row_slider,)


@app.cell(hide_code=True)
def _(mo):
    mo.md(r"""
    ### Einrückungen

    In Python werden Einrückungen verwendet, um Blöcke von Code zu definieren. Ein Block von Code ist eine Gruppe von Anweisungen, die zusammengehören. Zum Beispiel gehört der Code, der in einer Funktion definiert ist, zu einem Block. Mehr dazu später.

    ``` python
    Morgenroutine:
        Aufstehen
        Zähneputzen
        Frühstücken
    ```

    Aufstehen, Zähneputzen und Frühstücken gehören also alle zur Morgenroutine.

    ---
    """)
    return


@app.cell(hide_code=True)
def _(mo):
    mo.md(r"""
    ### Beispiel: Reihenfolge und Einrückung
    """)
    return


@app.cell(hide_code=True)
def _(clean_code, mo):
    syntax_example_code_box = mo.ui.code_editor(clean_code("""# Python führt die Zeilen von oben nach unten aus.
    print("Roboter startet")
    print("Roboter fährt los")
    print("Roboter stoppt")

    # Eingerückte Zeilen gehören zu einem Block.
    roboter_aktiv = True
    if roboter_aktiv == True:
        print("Der Roboter ist aktiv")
        print("Diese Zeile gehört noch zum if-Block")

    print("Diese Zeile gehört nicht mehr zum if-Block")"""), language="python")
    syntax_example_code_box
    return (syntax_example_code_box,)


@app.cell(hide_code=True)
def _(contextlib, io, mo, syntax_example_code_box):
    syntax_example_code_box_output_buffer = io.StringIO()
    with contextlib.redirect_stdout(syntax_example_code_box_output_buffer):
        try:
            exec(syntax_example_code_box.value)
        except Exception as e:
            print(f"Execution Error: {e}")

    mo.md(rf"""
    **Ausgabe**:
    ``` markdown
    {syntax_example_code_box_output_buffer.getvalue()}
    ```
    """)
    return


@app.cell(hide_code=True)
def _(mo):
    mo.md(r"""
    ### Aufgabe: Reihenfolge korrigieren
    """)
    return


@app.cell(hide_code=True)
def _(clean_code, mo):
    syntax_task_code_box = mo.ui.code_editor(clean_code("""# Ändere die Reihenfolge der print-Zeilen so, dass die Ausgabe logisch ist.
    # Danach: Ändere roboter_aktiv auf False. Was passiert mit dem eingerückten Block?

    print("3. Der Roboter stoppt")
    print("1. Der Roboter startet")
    print("2. Der Roboter fährt los")

    roboter_aktiv = True
    if roboter_aktiv == True:
        print("4. Status: aktiv")"""), language="python")
    syntax_task_code_box
    return (syntax_task_code_box,)


@app.cell(hide_code=True)
def _(contextlib, io, mo, render_task_output, syntax_task_code_box):
    syntax_task_code_box_output_buffer = io.StringIO()
    with contextlib.redirect_stdout(syntax_task_code_box_output_buffer):
        try:
            exec(syntax_task_code_box.value)
        except Exception as e:
            print(f"Execution Error: {e}")

    render_task_output(
        mo,
        syntax_task_code_box_output_buffer.getvalue(),
        """1. Der Roboter startet
    2. Der Roboter fährt los
    3. Der Roboter stoppt
    4. Status: aktiv""",
    )
    return


@app.cell(hide_code=True)
def datentypen(mo):
    null_vs_zero = mo.image(
        src="src/none_zero_KLOPAPIER.jpg",
        alt="NULL vs. 0",
        width="420px",
    )
    mo.md(f"""
    ## Datentypen und Variablen

    Um Daten in einem Programm zu speichern benutzt man Variablen, also einen benannten Ort im Speicher.
    In Python gibt es verschiedene Arten von Daten die man in Variablen speichern kann.

    Die wichtigsten sind:

    - **Integer**: ganze Zahlen, z. B. 1, 2, 3
    - **Float**: Kommazahlen, z. B. 1.0, 2.5, 3.14
    - **String**: Text, z. B. "Hallo", "Welt"
    - **Boolean**: Wahrheitswerte, z. B. True, False
    - **None**: Ein spezieller Wert, der bedeutet, dass keine Daten vorhanden sind.

    /// details | None vs. Zero

    {null_vs_zero}
    </br>
    `NULL` und `None` meinen hier dasselbe: Es gibt gerade keinen Wert.
    </br>
    `0` ist dagegen ein echter Zahlenwert, mit dem Python rechnen kann.

    ///
    """)
    return


@app.cell(hide_code=True)
def _(mo):
    mo.md(r"""
    ### Beispiel: Variablen und Datentypen
    """)
    return


@app.cell(hide_code=True)
def _(clean_code, mo):
    variable_code_box = mo.ui.code_editor(clean_code("""# Einer Variable einen Wert zuweisen ist sehr einfach:
    # Name = Wert

    zahl = 7.5

    # zahl hat jetzt den Wert 7.5
    # Um das zu überprüfen, können wir zwei einfache Befehle verwenden:

    print(zahl) # Gibt den Wert der Variable zahl aus
    print(type(zahl)) # Gibt den Datentyp der Variable zahl aus

    # Das Gleiche würde natürlich auch für andere Datentypen funktionieren:

    satz = "Hallo Welt!"

    print(satz) # Gibt den Wert der Variable satz aus
    print(type(satz)) # Gibt den Datentyp der Variable satz aus"""), language="python")
    variable_code_box
    return (variable_code_box,)


@app.cell(hide_code=True)
def _(contextlib, io, mo, variable_code_box):
    variable_code_box_output_buffer = io.StringIO()
    with contextlib.redirect_stdout(variable_code_box_output_buffer):
        try:
            exec(variable_code_box.value)
        except Exception as e:
            # Catch and print any syntax errors in the user's code
            print(f"Execution Error: {e}") 

    mo.md(rf"""
    **Ausgabe**:
    ``` markdown
    {variable_code_box_output_buffer.getvalue()}
    ```
    """)
    return


@app.cell
def _(mo):
    mo.md(r"""
    Für den Roboter müssen natürlich auch Variablen definiert werden.
    """)
    return


@app.cell(hide_code=True)
def _(mo):
    mo.md(r"""
    ### Aufgabe: Roboter-Variablen
    """)
    return


@app.cell(hide_code=True)
def _(clean_code, mo):
    robot_variables_code_box = mo.ui.code_editor(clean_code("""# Ändere die Werte und ergänze danach einen dritten Roboter.

    spieler_nummer = 3
    ziel_geschwindigkeit = 5.3
    roboter_name = "Ernst-Günther"

    zweite_spieler_nummer = spieler_nummer + 1
    zweite_ziel_geschwindigkeit = 3.14
    zweiter_roboter_name = "Ute"


    print(f"Der Roboter {roboter_name} hat die Spielernummer {spieler_nummer}")
    print(f"{roboter_name} hat eine Zielgeschwindigkeit von {ziel_geschwindigkeit} m/s.")
    print(f"Der zweite Roboter {zweiter_roboter_name} hat die Spielernummer {zweite_spieler_nummer}")
    print(f"{zweiter_roboter_name} hat eine Zielgeschwindigkeit von {zweite_ziel_geschwindigkeit} m/s.")"""), language="python")
    robot_variables_code_box
    return (robot_variables_code_box,)


@app.cell(hide_code=True)
def _(contextlib, io, mo, render_task_output, robot_variables_code_box):
    robot_variables_code_box_output_buffer = io.StringIO()
    with contextlib.redirect_stdout(robot_variables_code_box_output_buffer):
        try:
            exec(robot_variables_code_box.value)
        except Exception as e:
            # Catch and print any syntax errors in the user's code
            print(f"Execution Error: {e}") 

    render_task_output(
        mo,
        robot_variables_code_box_output_buffer.getvalue(),
        """Der Roboter Ernst-Günther hat die Spielernummer 3
    Ernst-Günther hat eine Zielgeschwindigkeit von 5.3 m/s.
    Der zweite Roboter Ute hat die Spielernummer 4
    Ute hat eine Zielgeschwindigkeit von 3.14 m/s.
    Der dritte Roboter Ada hat die Spielernummer 5
    Ada hat eine Zielgeschwindigkeit von 2.5 m/s.""",
    )
    return


@app.cell(hide_code=True)
def operatoren(mo):
    mo.md(r"""
    ## Operatoren und Ausdrücke

    Variablen sind, wie der Name sagt, *variabel*. Das bedeutet, dass man ihnen neue Werte zuweisen kann.

    Manche Werte wie die Spielernummer sollen sich während des Spiels nicht ändern können. Andere Werte wie die Ballposition oder die Laufgeschwindigkeit **müssen** sich ändern können.

    Um die Änderungen berechnen zu können, brauchen wir nun Operatoren. Operatoren sind Symbole oder Wörter, die eine Operation auf einem oder mehreren Operanden ausführen. Zum Beispiel ist der Plus-Operator (+) ein Operator, der zwei Variablen oder Werte addiert.

    **Die grundlegenden Operatoren in Python sind**:
    - Addition (+): addiert zwei Werte oder Variablen. Beispiel: `a + b`
    - Subtraktion (-): subtrahiert einen Wert oder eine Variable von einem anderen. Beispiel: `a - b`
    - Multiplikation (*): multipliziert zwei Werte oder Variablen. Beispiel: `a * b`
    - Division (/): dividiert einen Wert oder eine Variable durch einen anderen. Beispiel: `a / b`
    - Modulus (%): gibt den Rest einer Division zurück. Beispiel: `a % b`
    - Exponentiation (**): erhöht einen Wert oder eine Variable auf die Potenz eines anderen. Beispiel: `a ** b`
    - Vergleichsoperatoren: vergleicht zwei Werte oder Variablen und gibt einen Boolean zurück. Beispiel: `a == b`, `a != b`, `a > b`, `a < b`, `a >= b`, `a <= b`
    - Logische Operatoren: verknüpfen zwei oder mehr Bedingungen und geben einen Boolean zurück. Beispiel: `a and b`, `a or b`, `not a`

    Ausdrücke sind Kombinationen von Variablen, Operatoren und Werten, die einen neuen Wert ergeben. Zum Beispiel ist `a + b` ein Ausdruck, der die Werte von `a` und `b` addiert und das Ergebnis zurückgibt. Ein Ausdruck kann dann einer Variable zugewiesen werden, um das Ergebnis zu speichern. Zum Beispiel: `c = a + b` weist das Ergebnis der Addition von `a` und `b` der Variable `c` zu.
    """)
    return


@app.cell(hide_code=True)
def _(mo):
    mo.md(r"""
    ### Beispiel: Operatoren anwenden
    """)
    return


@app.cell(hide_code=True)
def _(clean_code, mo):
    operators_example_code_box = mo.ui.code_editor(clean_code("""# Operatoren berechnen aus vorhandenen Werten neue Werte.
    akku_start = 80
    verbrauch = 15
    akku_nach_fahrt = akku_start - verbrauch

    strecke_meter = 12
    zeit_sekunden = 3
    geschwindigkeit = strecke_meter / zeit_sekunden

    akku_kritisch = akku_nach_fahrt < 20
    schnell = geschwindigkeit > 3

    print(f"Akku nach der Fahrt: {akku_nach_fahrt}%")
    print(f"Geschwindigkeit: {geschwindigkeit} m/s")
    print(f"Akku kritisch? {akku_kritisch}")
    print(f"Schnell unterwegs? {schnell}")"""), language="python")
    operators_example_code_box
    return (operators_example_code_box,)


@app.cell(hide_code=True)
def _(contextlib, io, mo, operators_example_code_box):
    operators_example_code_box_output_buffer = io.StringIO()
    with contextlib.redirect_stdout(operators_example_code_box_output_buffer):
        try:
            exec(operators_example_code_box.value)
        except Exception as e:
            print(f"Execution Error: {e}")

    mo.md(rf"""
    **Ausgabe**:
    ``` markdown
    {operators_example_code_box_output_buffer.getvalue()}
    ```
    """)
    return


@app.cell(hide_code=True)
def _(mo):
    mo.md(r"""
    ### Aufgabe: Operatoren verändern

    Wähle `a` und `b` so, dass `e`, `f` und `g` alle `True` sind.
    """)
    return


@app.cell(hide_code=True)
def _(clean_code, mo):
    expression_code_box = mo.ui.code_editor(clean_code("""
    # Ändere a und b so, dass e, f und g am Ende True sind.
    # Tipp: Eine mögliche Lösung ist a = 6 und b = 9.
    a = 10
    b = 5

    c = a + b

    d = a * b

    e = a < b 

    f = d % 3 == 0

    g = e and f

    print(f"c: {c}, d: {d}, e: {e}, f: {f}, g: {g}")"""), language="python")
    expression_code_box
    return (expression_code_box,)


@app.cell(hide_code=True)
def _(contextlib, expression_code_box, io, mo, render_task_output):
    expression_code_box_output_buffer = io.StringIO()
    with contextlib.redirect_stdout(expression_code_box_output_buffer):
        try:
            exec(expression_code_box.value)
        except Exception as e:
            # Catch and print any syntax errors in the user's code
            print(f"Execution Error: {e}") 

    render_task_output(
        mo,
        expression_code_box_output_buffer.getvalue(),
        """c: 15, d: 54, e: True, f: True, g: True""",
    )
    return


@app.cell(hide_code=True)
def kontrollstrukturen(mo):
    mo.md(r"""
    ## Kontrollstrukturen (if, for, while)

    Am Anfang haben wir gesagt, dass Python-Programme von oben nach unten ausgeführt werden. Es gibt jedoch einige Ausnahmen, die es ermöglichen, dass Code nicht in der Reihenfolge ausgeführt wird, in der er geschrieben ist. Diese Ausnahmen sind Kontrollstrukturen, die es ermöglichen, dass Code nur unter bestimmten Bedingungen ausgeführt wird oder dass Code mehrmals ausgeführt wird.

    Die drei Kontrollstrukturen, die wir in diesem Tutorial behandeln werden, sind:
    - **if**: Mit der if-Anweisung kann Code mit einer bestimmten Bedingung ausgeführt werden.
    ```python
    if a > b:
      print("a ist größer als b")
    else:
      print("b ist größer als a")

    ```

    - **for**: Mit der for-Schleife kann Code eine bestimmte Anzahl von Malen ausgeführt werden.
    ```python
    for i in range(5):
      print(i)
    ```

    - **while**: Mit der while-Schleife kann Code so lange ausgeführt werden, wie eine bestimmte Bedingung erfüllt ist.
    ```python

    while a > b:
      print("a ist größer als b")
      a = a - 1
    ```
    """)
    return


@app.cell(hide_code=True)
def _(mo):
    mo.md(r"""
    ### Beispiel: if, for und while
    """)
    return


@app.cell(hide_code=True)
def _(clean_code, mo):
    conditions_example_code_box = mo.ui.code_editor(clean_code("""# if entscheidet, welcher Code ausgeführt wird.
    abstand_zum_ball = 1.2

    if abstand_zum_ball < 0.5:
        print("Ball ist nah")
    else:
        print("Ball ist noch weit weg")

    # for wiederholt Code für eine feste Anzahl von Durchläufen.
    for sekunde in range(3):
        print(f"Sekunde {sekunde}")

    # while wiederholt Code, solange die Bedingung wahr ist.
    akku = 30
    while akku > 0:
        print(f"Akku: {akku}%")
        akku = akku - 10"""), language="python")
    conditions_example_code_box
    return (conditions_example_code_box,)


@app.cell(hide_code=True)
def _(conditions_example_code_box, contextlib, io, mo):
    conditions_example_code_box_output_buffer = io.StringIO()
    with contextlib.redirect_stdout(conditions_example_code_box_output_buffer):
        try:
            exec(conditions_example_code_box.value)
        except Exception as e:
            print(f"Execution Error: {e}")

    mo.md(rf"""
    **Ausgabe**:
    ``` markdown
    {conditions_example_code_box_output_buffer.getvalue()}
    ```
    """)
    return


@app.cell(hide_code=True)
def _(mo):
    mo.md(r"""
    ### Aufgabe: Kontrollstrukturen anpassen
    """)
    return


@app.cell(hide_code=True)
def _(clean_code, mo):
    conditions_code_box = mo.ui.code_editor(clean_code("""# Setze roboter_geschwindigkeit auf 0.01 und beobachte die erste Ausgabe.
    # Ändere danach die Anzahl der Schleifendurchläufe in range(...).

    roboter_geschwindigkeit = 5.3

    bewegungs_grenzwert = 0.05

    if roboter_geschwindigkeit < bewegungs_grenzwert:
        print("Der Roboter bewegt sich nicht")
    else:
        print("Der Roboter bewegt sich")

    for i in range(5):
        roboter_geschwindigkeit = roboter_geschwindigkeit + 1
        print(f"Geschwindigkeit = {roboter_geschwindigkeit:.2f} m/s")
        # :.2f ist eine Schreibweise, um die Zahl auf 2 Nachkommastellen zu runden

    while roboter_geschwindigkeit > bewegungs_grenzwert:
        print(f"Der Roboter bewegt sich mit {roboter_geschwindigkeit:.2f} m/s")
        roboter_geschwindigkeit = roboter_geschwindigkeit * 0.5"""), language="python")
    conditions_code_box
    return (conditions_code_box,)


@app.cell(hide_code=True)
def _(conditions_code_box, contextlib, io, mo, render_task_output):
    conditions_code_box_output_buffer = io.StringIO()
    with contextlib.redirect_stdout(conditions_code_box_output_buffer):
        try:
            exec(conditions_code_box.value)
        except Exception as e:
            # Catch and print any syntax errors in the user's code
            print(f"Execution Error: {e}") 

    render_task_output(
        mo,
        conditions_code_box_output_buffer.getvalue(),
        """Der Roboter bewegt sich nicht
    Geschwindigkeit = 1.01 m/s
    Geschwindigkeit = 2.01 m/s
    Der Roboter bewegt sich mit 2.01 m/s
    Der Roboter bewegt sich mit 1.00 m/s
    Der Roboter bewegt sich mit 0.50 m/s
    Der Roboter bewegt sich mit 0.25 m/s
    Der Roboter bewegt sich mit 0.13 m/s
    Der Roboter bewegt sich mit 0.06 m/s""",
    )
    return


@app.cell(hide_code=True)
def funktionen(mo):
    mo.md(r"""
    ## Funktionen

    Je größer die Programme werden, desto unübersichtlicher kann der Code werden. Funktionen sind eine Möglichkeit, um Code zu organisieren und wiederverwendbar zu machen. Eine Funktion ist ein benannter Block von Code, der eine bestimmte Aufgabe ausführt. Funktionen können Parameter haben, die es ermöglichen, dass die Funktion mit unterschiedlichen Daten arbeiten kann. Funktionen können auch einen Rückgabewert haben, der es ermöglicht, dass die Funktion ein Ergebnis zurückgibt.

    Wenn ein Stück Code mehrmals benutzt wird, ist es meistens sinnvoll, diesen Code in eine Funktion zu packen. Zum Beispiel brauchen wir für die Bewegung des Roboters immer wieder den gleichen Code, um die Geschwindigkeit zu berechnen. Es wäre also sinnvoll, diesen Code in eine Funktion zu packen, damit wir ihn nicht jedes Mal neu schreiben müssen.

    Um eine Funktion zu definieren brauchen wir das Schlüsselwort `def`, gefolgt von dem Namen der Funktion und einer Klammer, in der die Parameter definiert werden können. Der Code, der zur Funktion gehört, muss eingerückt sein.

    ``` python
    def nameDerFunktion():
        ...
        Code
        ...
        return Ergebnis
    ```
    """)
    return


@app.cell(hide_code=True)
def _(mo):
    mo.md(r"""
    ### Beispiel: Funktionen definieren
    """)
    return


@app.cell(hide_code=True)
def _(clean_code, mo):
    function_code_box = mo.ui.code_editor(
        clean_code("""# Eine Funktion fasst wiederverwendbaren Code zusammen.

    def distanz_berechnen(x1, x2):
        distanz = abs(x1 - x2) # abs() ist eine Funktion von Python selbst, die den absoluten Wert einer Zahl zurückgibt
        return distanz

    print(distanz_berechnen(5, 3))

    def nummer_zu_name(nummer):
        if nummer == 1:
            return "Ernst-Günther"
        elif nummer == 2:
            return "Ute"
        else:
            return "Unbekannt"
    """), language="python")
    function_code_box
    return (function_code_box,)


@app.cell(hide_code=True)
def _(contextlib, function_code_box, io, mo):
    function_code_box_output_buffer = io.StringIO()
    with contextlib.redirect_stdout(function_code_box_output_buffer):
        try:
            exec(function_code_box.value)
        except Exception as e:
            # Catch and print any syntax errors in the user's code
            print(f"Execution Error: {e}") 

    mo.md(rf"""
    **Ausgabe**:
    ``` markdown
    {function_code_box_output_buffer.getvalue()}
    ```
    ---
    """)
    return


@app.cell(hide_code=True)
def _(mo):
    mo.md(r"""
    ### Aufgabe: Fahrzeit-Funktion
    """)
    return


@app.cell(hide_code=True)
def _(clean_code, mo):
    function_task_code_box = mo.ui.code_editor(
        clean_code("""# Ergänze die Funktion so, dass sie die Fahrzeit berechnet.
    # Formel: Zeit = Strecke / Geschwindigkeit

    def fahrzeit_berechnen(strecke_meter, geschwindigkeit_meter_pro_sekunde):
        zeit = 0
        return zeit

    print(fahrzeit_berechnen(12, 3))

    # Zusatzaufgabe: Gib bei Geschwindigkeit 0 den Text "steht" zurück.
    print(fahrzeit_berechnen(12, 0))"""), language="python")
    function_task_code_box
    return (function_task_code_box,)


@app.cell(hide_code=True)
def _(contextlib, function_task_code_box, io, mo, render_task_output):
    function_task_code_box_output_buffer = io.StringIO()
    with contextlib.redirect_stdout(function_task_code_box_output_buffer):
        try:
            exec(function_task_code_box.value)
        except Exception as e:
            print(f"Execution Error: {e}")

    render_task_output(
        mo,
        function_task_code_box_output_buffer.getvalue(),
        """4.0
    steht""",
    )
    return


@app.cell(hide_code=True)
def datenstrukturen2(mo):
    mo.md(r"""
    ## Erweiterte Datenstrukturen (Listen, Tupel)

    Wenn man mehrere Roboter hat, kann man natürlich für jeden Roboter eigene Variablen anlegen, allerdings ist das nicht sehr effizient. Es gibt in Python verschiedene Möglichkeiten, um mehrere Werte kompakt in einer Variable zu speichern.

    Die wichtigsten Datenstrukturen, die wir in diesem Tutorial behandeln werden, sind:
    - **Listen**: Eine Liste ist eine geordnete Sammlung von Werten, die in eckigen Klammern [ ] geschrieben wird. Listen können Werte unterschiedlichen Typs enthalten und sind veränderbar, das heißt, man kann Einträge später ändern, hinzufügen oder löschen.
    - **Tupel**: Ein Tupel ist eine geordnete Sammlung von Werten, die in runden Klammern ( ) geschrieben wird. Tupel können Werte unterschiedlichen Typs enthalten, sind aber unveränderbar, das heißt, man kann Einträge nach dem Erstellen nicht mehr ändern.

    ``` python
    namen_liste = ["Ernst-Günther", "Ute"] # Liste von Strings

    roboter_tupel = ("Ernst-Günther", 1) # Tupel von einem String und einem Integer

    roboter_liste = [("Ernst-Günther", 1), ("Ute", 2)] # Liste von Tupeln
    ```

    Um die Werte einer Liste oder eines Tupels abzurufen benutzt man eckige Klammern [ ] und den Index des Werts.
    Indizes von Werten starten bei 0, das heißt der erste Wert hat den Index 0, der zweite Wert hat den Index 1, usw.

    [0, 1, 2, 3...]
    """)
    return


@app.cell(hide_code=True)
def _(mo):
    mo.md(r"""
    ### Beispiel: Listen und Tupel verwenden
    """)
    return


@app.cell
def _(clean_code, mo):
    lists_code_box = mo.ui.code_editor(clean_code("""# In einer Liste können mehrere Roboter gespeichert werden.

    roboter_liste = [("Ernst-Günther", 1), ("Ute", 2)]

    print(roboter_liste[0])

    print(roboter_liste[1][0])

    # Über Listen kann man auch mit einer for-Schleife iterieren. Dabei wird nacheinander jedes Element aus der Liste eingesetzt.
    for roboter in roboter_liste: 
        name = roboter[0]
        nummer = roboter[1]
        print(f"Der Roboter {name} hat die Spielernummer {nummer}")
    """), language="python")
    lists_code_box
    return (lists_code_box,)


@app.cell(hide_code=True)
def _(contextlib, io, lists_code_box, mo):
    lists_code_box_output_buffer = io.StringIO()
    with contextlib.redirect_stdout(lists_code_box_output_buffer):
        try:
            exec(lists_code_box.value)
        except Exception as e:
            # Catch and print any syntax errors in the user's code
            print(f"Execution Error: {e}") 

    mo.md(rf"""
    **Ausgabe**:
    ``` markdown
    {lists_code_box_output_buffer.getvalue()}
    ```
    ---
    """)
    return


@app.cell(hide_code=True)
def _(mo):
    mo.md(r"""
    ### Aufgabe: Roboterliste erweitern
    """)
    return


@app.cell(hide_code=True)
def _(clean_code, mo):
    lists_task_code_box = mo.ui.code_editor(clean_code("""# Ergänze einen dritten Roboter und gib alle Roboter aus.
    # Danach: Ändere die Spielernummer von Ute auf 5.

    roboter_liste = [("Ernst-Günther", 1), ("Ute", 2)]

    # Tipp: Mit append(...) kannst du einer Liste einen neuen Eintrag hinzufügen.
    # roboter_liste.append(("Ada", 3))

    for name, nummer in roboter_liste:
        print(f"{name}: {nummer}")"""), language="python")
    lists_task_code_box
    return (lists_task_code_box,)


@app.cell(hide_code=True)
def _(contextlib, io, lists_task_code_box, mo, render_task_output):
    lists_task_code_box_output_buffer = io.StringIO()
    with contextlib.redirect_stdout(lists_task_code_box_output_buffer):
        try:
            exec(lists_task_code_box.value)
        except Exception as e:
            print(f"Execution Error: {e}")

    render_task_output(
        mo,
        lists_task_code_box_output_buffer.getvalue(),
        """Ernst-Günther: 1
    Ute: 5
    Ada: 3""",
    )
    return


@app.cell(hide_code=True)
def module(mo):
    mo.md(r"""
    ## Pakete

    Vor allem für komplexere Programme brauchen wir auch komplexere Funktionen. Diese Funktionen müssen wir nicht alle selbst schreiben, sondern können sie auch aus Paketen importieren. Pakete können sehr verschiedene Funktionen enthalten, zum Beispiel welche, die einfache Matheoperationen durchführen oder welche, die es ermöglichen, Bilder zu verarbeiten.

    Zum Importieren eines Paketes benötigt man das Schlüsselwort `import`, gefolgt von dem Namen des Paketes.

    Zum Beispiel:

    ```python
    import math
    ```

    Nach dem Importieren eines Paketes kann man die Funktionen des Paketes verwenden, indem man den Namen des Paketes gefolgt von einem Punkt und dem Namen der Funktion schreibt.

    Zum Beispiel:

    ``` python
    print(math.sqrt(16)) # Gibt die Quadratwurzel von 16 zurück
    ```

    Manche Pakete haben auch zu lange Namen, um sie immer wieder zu schreiben. Deswegen kann man sie auch mit einem kürzeren Namen importieren.

    Zum Beispiel:

    ``` python
    import numpy as np

    np.array([1, 2, 3])
    ```

    Das Schlüsselwort *as* ermöglicht es also, einen anderen Namen für das Paket auszuwählen.
    Manchmal ist es auch möglich, nur bestimmte Funktionen aus einem Paket zu importieren, anstatt das ganze Paket zu importieren.

    Zum Beispiel:

    ```python
    from math import sqrt

    print(sqrt(16)) # Die gleiche Funktion ist nun direkt verfügbar und andere Funktionen aus dem math Paket nicht
    ```
    """)
    return


@app.cell(hide_code=True)
def _(mo):
    mo.md(r"""
    ### Beispiel: Pakete importieren
    """)
    return


@app.cell(hide_code=True)
def _(clean_code, mo):
    package_code_box = mo.ui.code_editor(
    clean_code("""# Funktionen können aus Paketen importiert werden.

    from math import sqrt

    print(sqrt(16))

    import math as mathematik

    print(mathematik.sqrt(16))

    from math import sqrt as quadratwurzel

    print(quadratwurzel(36))
    """), language="python")
    package_code_box
    return (package_code_box,)


@app.cell(hide_code=True)
def _(contextlib, io, mo, package_code_box):
    package_code_box_output_buffer = io.StringIO()
    with contextlib.redirect_stdout(package_code_box_output_buffer):
        try:
            exec(package_code_box.value)
        except Exception as e:
            # Catch and print any syntax errors in the user's code
            print(f"Execution Error: {e}") 

    mo.md(rf"""
    **Ausgabe**:
    ``` markdown
    {package_code_box_output_buffer.getvalue()}
    ```
    ---
    """)
    return


@app.cell(hide_code=True)
def _(mo):
    mo.md(r"""
    ### Aufgabe: Paketfunktion nutzen
    """)
    return


@app.cell(hide_code=True)
def _(clean_code, mo):
    package_task_code_box = mo.ui.code_editor(
        clean_code("""# Importiere die passende Funktion und berechne die Diagonale.
    # Formel: diagonale = sqrt(breite ** 2 + hoehe ** 2)

    from math import sqrt

    breite = 3
    hoehe = 4
    diagonale = 0

    print(diagonale)"""), language="python")
    package_task_code_box
    return (package_task_code_box,)


@app.cell(hide_code=True)
def _(contextlib, io, mo, package_task_code_box, render_task_output):
    package_task_code_box_output_buffer = io.StringIO()
    with contextlib.redirect_stdout(package_task_code_box_output_buffer):
        try:
            exec(package_task_code_box.value)
        except Exception as e:
            print(f"Execution Error: {e}")

    render_task_output(
        mo,
        package_task_code_box_output_buffer.getvalue(),
        """5.0""",
    )
    return


@app.cell(hide_code=True)
def numpy(mo):
    mo.md(r"""
    ## NumPy basics

    **Hinweis:** Dieser Abschnitt ist anspruchsvoller als die Grundlagen davor. Für jüngere Gruppen kann er als Bonus- oder Projektteil genutzt werden.

    Wir gehen NumPy jetzt langsamer in vier Schritten an:

    1. **Arrays wie Listen:** Werte speichern und mit Indizes abrufen.
    2. **Rechnen mit Arrays:** Eine Rechnung kann auf alle Werte gleichzeitig angewendet werden.
    3. **Bilder als Arrays:** Pixel können als Zahlen gespeichert werden.
    4. **Bonus - Slicing:** Aus Listen oder Arrays werden gezielt Bereiche herausgeschnitten.

    ### 1. Arrays wie Listen

    Ein NumPy-Array ist am Anfang ähnlich wie eine Python-Liste: Man kann mehrere Werte speichern und einzelne Werte über einen Index abrufen.

    ``` python
    import numpy as np

    roboter_ladungen = np.array([0.82, 0.33, 0.65, 0.70, 0.99])
    print(roboter_ladungen[0])
    ```

    Wichtig: NumPy-Arrays sind besonders nützlich, wenn viele Zahlen verarbeitet werden sollen. Deshalb werden sie häufig in Technik, Datenanalyse, Robotik und Bildverarbeitung genutzt.
    """)
    return


@app.cell(hide_code=True)
def _(mo):
    mo.md(r"""
    ### Beispiel: NumPy-Arrays wie Listen
    """)
    return


@app.cell(hide_code=True)
def _(clean_code, mo):
    nparray_example_code_box = mo.ui.code_editor(
        clean_code("""# Ein NumPy-Array kann ähnlich wie eine Liste benutzt werden.
    import numpy as np

    roboter_ladungen = np.array([0.82, 0.33, 0.65, 0.70, 0.99])

    print(roboter_ladungen)
    print(roboter_ladungen[0])
    print(roboter_ladungen[1])
    print(len(roboter_ladungen))"""), language="python")
    nparray_example_code_box
    return (nparray_example_code_box,)


@app.cell(hide_code=True)
def _(contextlib, io, mo, np, nparray_example_code_box):
    nparray_example_code_box_output_buffer = io.StringIO()
    with contextlib.redirect_stdout(nparray_example_code_box_output_buffer):
        try:
            exec(nparray_example_code_box.value, {"np": np})
        except Exception as e:
            print(f"Execution Error: {e}")

    mo.md(rf"""
    **Ausgabe**:
    ``` markdown
    {nparray_example_code_box_output_buffer.getvalue()}
    ```
    """)
    return


@app.cell(hide_code=True)
def _(mo):
    mo.md(r"""
    ### 2. Rechnen mit Arrays

    Bei NumPy kann eine Rechnung auf alle Werte im Array gleichzeitig angewendet werden. Das ist der große Unterschied zu normalen Python-Listen.

    ### Beispiel: Rechnen mit NumPy-Arrays
    """)
    return


@app.cell(hide_code=True)
def _(clean_code, mo):
    nparray_calculation_code_box = mo.ui.code_editor(
        clean_code("""# NumPy kann mit ganzen Arrays auf einmal rechnen.
    import numpy as np

    roboter_ladungen = np.array([0.82, 0.33, 0.65, 0.70, 0.99])

    durchschnitt = np.mean(roboter_ladungen)
    kleinste_ladung = np.min(roboter_ladungen)
    ladungen_in_prozent = roboter_ladungen * 100

    print(f"Durchschnitt: {durchschnitt:.2f}")
    print(f"Kleinste Ladung: {kleinste_ladung:.2f}")
    print(ladungen_in_prozent)"""), language="python")
    nparray_calculation_code_box
    return (nparray_calculation_code_box,)


@app.cell(hide_code=True)
def _(contextlib, io, mo, np, nparray_calculation_code_box):
    nparray_calculation_code_box_output_buffer = io.StringIO()
    with contextlib.redirect_stdout(nparray_calculation_code_box_output_buffer):
        try:
            exec(nparray_calculation_code_box.value, {"np": np})
        except Exception as e:
            print(f"Execution Error: {e}")

    mo.md(rf"""
    **Ausgabe**:
    ``` markdown
    {nparray_calculation_code_box_output_buffer.getvalue()}
    ```
    """)
    return


@app.cell(hide_code=True)
def _(mo):
    mo.md(r"""
    ### 3. Bilder als Arrays

    Ein Farbbild kann als Array gespeichert werden. Jeder Pixel besteht hier aus drei Zahlen: Rot, Grün und Blau.

    ### Aufgabe: Roboter-Smiley

    Bei `ladung = 100` soll ein grünes lächelndes Gesicht erscheinen. Bei `ladung = 10` soll ein rotes trauriges Gesicht erscheinen.
    """)
    return


@app.cell(hide_code=True)
def _(clean_code, mo):
    nparray_code_box = mo.ui.code_editor(
        clean_code("""# Jeder Pixel im Bild unten ist ein NumPy-Array mit drei Einträgen [Rot, Grün, Blau].
    # Schreibe eine if-Abfrage, die je nach Ladung ein rotes trauriges oder grünes lächelndes Gesicht erzeugt.

    # Die Variable 'ladung' gibt an, wie viel Strom ein Roboter noch hat.
    ladung = 100

    # Wenn die Ladung eines Roboters unter 20 ist, sollte er geladen werden.
    # In diesem Fall soll der Smiley wie unten traurig gucken und rot sein.

    # Wenn die Ladung über 20 ist, sollte der Smiley lächeln und grün sein.
    # Das aktuelle Bild ist absichtlich nur ein Startpunkt für die Aufgabe.

    bild = np.array([
        [[0.0, 0.0, 0.0], [0.0, 0.0, 0.0], [0.0, 0.0, 0.0], [0.0, 0.0, 0.0], [0.0, 0.0, 0.0], [0.0, 0.0, 0.0], [0.0, 0.0, 0.0]],
        [[0.0, 0.0, 0.0], [0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 0.0], [0.0, 0.0, 0.0]],
        [[0.0, 0.0, 0.0], [0.0, 0.0, 0.0], [0.0, 0.0, 0.0], [0.0, 0.0, 0.0], [0.0, 0.0, 0.0], [0.0, 0.0, 0.0], [0.0, 0.0, 0.0]],
        [[0.0, 0.0, 0.0], [0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 0.0], [0.0, 0.0, 0.0]],
        [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 0.0], [0.0, 0.0, 0.0], [0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 0.0]],
        [[0.0, 0.0, 0.0], [0.0, 0.0, 0.0], [0.0, 0.0, 0.0], [0.0, 0.0, 0.0], [0.0, 0.0, 0.0], [0.0, 0.0, 0.0], [0.0, 0.0, 0.0]],
    ])"""), language="python")
    nparray_code_box
    return (nparray_code_box,)


@app.cell(hide_code=True)
def _(contextlib, io, mo, np, nparray_code_box, plt):
    nparray_code_box_output_buffer = io.StringIO()
    plt.clf()

    def smiley_bild(ladung):
        farbe = np.array([0.0, 1.0, 0.0]) if ladung >= 20 else np.array([1.0, 0.0, 0.0])
        bild = np.zeros((6, 7, 3))
        bild[1, 2] = farbe
        bild[1, 4] = farbe
        if ladung >= 20:
            bild[3, 1] = farbe
            bild[4, 2:5] = farbe
            bild[3, 5] = farbe
        else:
            bild[3, 2:5] = farbe
            bild[4, 1] = farbe
            bild[4, 5] = farbe
        return bild

    with contextlib.redirect_stdout(nparray_code_box_output_buffer):
        try:
            namespace = {"np": np}
            exec(nparray_code_box.value, namespace)
            bild = namespace["bild"]
            ladung = namespace.get("ladung", 100)
            ziel_bild = smiley_bild(ladung)

            if bild.shape == ziel_bild.shape:
                unterschiede = int(np.sum(np.any(np.abs(bild - ziel_bild) > 1e-9, axis=2)))
                vergleich = "✅ Das Bild stimmt mit der Zielausgabe überein." if unterschiede == 0 else f"⚠️ {unterschiede} Pixel unterscheiden sich von der Zielausgabe."
            else:
                vergleich = f"⚠️ Die Bildgröße stimmt nicht. Aktuell: {bild.shape}, Ziel: {ziel_bild.shape}."

            bild_gross = np.repeat(np.repeat(bild, 50, axis=0), 50, axis=1)
            ziel_bild_gross = np.repeat(np.repeat(ziel_bild, 50, axis=0), 50, axis=1)
            bild_anzeige = mo.vstack([
                mo.hstack([
                    mo.vstack([
                        mo.md("**Aktuelle Ausgabe:**"),
                        mo.image(bild_gross),
                    ]),
                    mo.vstack([
                        mo.md("**Zielausgabe:**"),
                        mo.image(ziel_bild_gross),
                    ]),
                ]),
                mo.md(f"**Vergleich:**\n\n{vergleich}"),
            ])
        except Exception as e:
            print(f"Execution Error: {e}")
            bild_anzeige = mo.md(f"**Aktuelle Ausgabe:**\n``` markdown\nExecution Error: {e}\n```")

    bild_anzeige
    return


@app.cell(hide_code=True)
def _(mo):
    mo.md(r"""
    ---

    ### 4. Bonus: Fortgeschrittenes Indexen

    Um einen Wert aus einer Python-Liste zu bekommen haben wir vorhin die eckige Klammer [ ] genutzt und den Index (Position) des Wertes angegeben den wir haben wollten. Wenn wir eine Liste von Listen haben haben wir uns zuerst die innere Liste geholt und dann erst den Index des Wertes in der inneren Liste angegeben.

    ``` python
    komplexe_liste = [[1, 2, 3], [4, 5, 6], [7, 8, 9]]

    print(komplexe_liste[0][1]) # Wir gehen in die Liste mit dem Index 0 und holen dann den Wert mit dem Index 1
    ```

    Aber was ist wenn wir mehrere Werte haben wollen?

    ``` python
    print(komplexe_liste[:])
    ```

    Um mehrere Werte zu bekommen können wir den Index durch einen Doppelpunkt : ersetzen. Das bedeutet, dass wir alle Werte von der Liste haben wollen. Aber es gibt noch mehr was man mit dem Doppelpunkt machen kann.

    Ein einzelner Index ist eine einfache Zahl. Ein Doppelpunkt ist eine Art von Index, der es ermöglicht, einen Bereich von Werten zu spezifizieren. Der Bereich, den wir haben wollen, wird durch drei Werte definiert:

    ``` python
    Startwert : Endwert : Schrittweite
    ```

    Am einfachsten zu verstehen ist das Prinzip an einer einfachen Liste:
    """)
    return


@app.cell(hide_code=True)
def _(mo):
    mo.md(r"""
    ### Beispiel: Slicing mit Listen
    """)
    return


@app.cell
def _(clean_code, mo):
    slicing_code_box = mo.ui.code_editor(
    clean_code("""# Slicing holt mehrere Werte aus einer Liste.

    liste = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9]

    print(liste[2:5:1]) # Gibt die Werte von Index 2 bis Index 4 zurück (der Endwert ist exklusiv)
    """), language="python")
    slicing_code_box
    return (slicing_code_box,)


@app.cell(hide_code=True)
def _(contextlib, io, mo, slicing_code_box):
    slicing_code_box_output_buffer = io.StringIO()
    with contextlib.redirect_stdout(slicing_code_box_output_buffer):
        try:
            exec(slicing_code_box.value)
        except Exception as e:
            # Catch and print any syntax errors in the user's code
            print(f"Execution Error: {e}") 

    mo.md(rf"""
    **Ausgabe**:
    ``` markdown
    {slicing_code_box_output_buffer.getvalue()}
    ```
    ---
    """)
    return


@app.cell(hide_code=True)
def _(mo):
    mo.md(r"""
    Python guckt wie viele Doppelpunkte es gibt. Wenn es keinen gibt, dann ist es ein einfacher Index. Wenn es einen gibt, dann ist es ein Bereichsindex. Wenn es zwei gibt, dann ist es ein Bereichsindex mit Schrittweite.

    Aber zwischen Doppelpunkten müssen auch keine Zahlen stehen. Wenn kein Startwert angegeben ist, dann wird automatisch bei Index 0 angefangen. Wenn kein Endwert angegeben ist, dann wird automatisch bis zum Ende der Liste gegangen. Wenn keine Schrittweite angegeben ist, dann wird automatisch eine Schrittweite von 1 angenommen.

    ``` python
    komplexe_liste[:] == komplexe_liste[::] == komplexe_liste

    # Weil wir nur die Standardwerte eingegeben haben, ist es egal ob wir sie explizit angeben oder nicht
    # In allen Fällen hier holen wir uns die ganze Liste
    komplexe_liste[0:] == komplexe_liste[0::1] == komplexe_liste
    ```

    Wenn wir mehrdimensionale NumPy-Arrays haben, dann können wir das gleiche Prinzip auch auf die anderen Dimensionen anwenden.
    Allerdings brauchen wir dafür noch einen weiteren Trick, nämlich das Komma. Das Komma ermöglicht es uns, die verschiedenen Dimensionen zu trennen.

    ``` python
    komplexes_array = np.array([[1, 2, 3],
                               [4, 5, 6],
                               [7, 8, 9]])

    # Unser Ziel ist es an die Zahl 5 zu kommen

    print(komplexes_array[1][1]) # Wir gehen in die Liste mit dem Index 1 und holen dann den Wert mit dem Index 1

    # Das gleiche können wir aber auch mit einem einzigen Index machen, indem wir die Dimensionen mit einem Komma trennen

    print(komplexes_array[1, 1]) # Wir gehen in die Liste mit dem Index 1 und holen dann den Wert mit dem Index 1
    ```

    Das alleine ist noch nicht viel anders, aber es ermöglicht es uns, eine Spalte an Werten zu holen.

    ``` python
    # Wir holen uns alle Zeilen
    # Und nehmen aus jeder das Element mit dem Index 1
    print(komplexes_array[:, 1] == [2, 5, 8])
    ```
    """)
    return


@app.cell(hide_code=True)
def _(mo):
    mo.md(r"""
    ### Aufgabe: Mehrdimensionales Slicing
    """)
    return


@app.cell(hide_code=True)
def _(clean_code, mo):
    multidimensional_slicing_code_box = mo.ui.code_editor(
    clean_code("""# Passe den Slice so an, dass die inneren vier Werte des Arrays ausgegeben werden.

    # Also:
    # [[ 6,  7], 
    #  [10, 11]]

    komplexes_array = np.array([[ 1,  2,  3,  4],
                               [ 5,  6,  7,  8], 
                               [ 9, 10, 11, 12], 
                               [13, 14, 15, 16]])

    innere_box = komplexes_array[1, 1]

    print(innere_box)
    """), language="python")
    multidimensional_slicing_code_box
    return (multidimensional_slicing_code_box,)


@app.cell(hide_code=True)
def _(
    contextlib,
    io,
    mo,
    multidimensional_slicing_code_box,
    np,
    render_task_output,
):
    multidimensional_slicing_code_box_buffer = io.StringIO()
    with contextlib.redirect_stdout(multidimensional_slicing_code_box_buffer):
        try:
            exec(multidimensional_slicing_code_box.value, {"np": np})
        except Exception as e:
            # Catch and print any syntax errors in the user's code
            print(f"Execution Error: {e}") 

    render_task_output(
        mo,
        multidimensional_slicing_code_box_buffer.getvalue(),
        """[[ 6  7]
     [10 11]]""",
    )
    return


@app.cell(hide_code=True)
def _(mo):
    mo.md(r"""
    ---

    ## Abschlussaufgaben: Mini-Projekte

    Wähle mindestens eine Aufgabe. Die Schwierigkeitsstufen helfen dabei, dass alle eine passende Herausforderung finden:

    - 🟢 **Basis:** Wiederholt die wichtigsten Grundlagen.
    - 🟡 **Challenge:** Kombiniert mehrere Themen.
    - 🔴 **Profi / Bonus:** Nutzt mehrere Konzepte zusammen, auch NumPy.
    """)
    return


@app.cell(hide_code=True)
def _(mo):
    mo.md(r"""
    ### 🟢 Basis: Startfreigabe für den Roboter

    Verwende Variablen, Operatoren und `if`, um zu entscheiden, ob der Roboter starten darf.
    """)
    return


@app.cell(hide_code=True)
def _(clean_code, mo):
    final_basic_task_code_box = mo.ui.code_editor(clean_code("""# Der Roboter darf starten, wenn der Akku über 20 ist
    # und kein Hindernis erkannt wurde.

    akku = 80
    hindernis_erkannt = False

    darf_starten = False

    if darf_starten:
        print("Roboter darf starten")
    else:
        print("Roboter muss warten")"""), language="python")
    final_basic_task_code_box
    return (final_basic_task_code_box,)


@app.cell(hide_code=True)
def _(contextlib, final_basic_task_code_box, io, mo, render_task_output):
    final_basic_task_code_box_output_buffer = io.StringIO()
    with contextlib.redirect_stdout(final_basic_task_code_box_output_buffer):
        try:
            exec(final_basic_task_code_box.value)
        except Exception as e:
            print(f"Execution Error: {e}")

    render_task_output(
        mo,
        final_basic_task_code_box_output_buffer.getvalue(),
        """Roboter darf starten""",
    )
    return


@app.cell(hide_code=True)
def _(mo):
    mo.md(r"""
    ### 🟡 Challenge: Team-Status ausgeben

    Verwende eine Liste, eine Schleife und eine Funktion. Jeder Roboter soll anhand seines Akkus einen Status bekommen.
    """)
    return


@app.cell(hide_code=True)
def _(clean_code, mo):
    final_challenge_task_code_box = mo.ui.code_editor(clean_code("""# Ergänze die Funktion status_bestimmen(...).
    # Ab 20 Prozent Akku ist ein Roboter "bereit", sonst muss er "laden".

    roboter_team = [("Ernst-Günther", 80), ("Ute", 12), ("Ada", 55)]

    def status_bestimmen(akku):
        return "unbekannt"

    for name, akku in roboter_team:
        status = status_bestimmen(akku)
        print(f"{name}: {status}")"""), language="python")
    final_challenge_task_code_box
    return (final_challenge_task_code_box,)


@app.cell(hide_code=True)
def _(contextlib, final_challenge_task_code_box, io, mo, render_task_output):
    final_challenge_task_code_box_output_buffer = io.StringIO()
    with contextlib.redirect_stdout(final_challenge_task_code_box_output_buffer):
        try:
            exec(final_challenge_task_code_box.value)
        except Exception as e:
            print(f"Execution Error: {e}")

    render_task_output(
        mo,
        final_challenge_task_code_box_output_buffer.getvalue(),
        """Ernst-Günther: bereit
    Ute: laden
    Ada: bereit""",
    )
    return


@app.cell(hide_code=True)
def _(mo):
    mo.md(r"""
    ### 🔴 Profi / Bonus: Sensorwerte mit NumPy prüfen

    Verwende NumPy, um Sensorwerte auszuwerten. Der Roboter soll stoppen, wenn ein Hindernis näher als `0.30` Meter ist.
    """)
    return


@app.cell(hide_code=True)
def _(clean_code, mo):
    final_pro_task_code_box = mo.ui.code_editor(clean_code("""# Ergänze die Berechnungen mit NumPy.

    import numpy as np

    abstaende = np.array([0.80, 0.45, 0.25, 0.70])

    kleinster_abstand = 0
    durchschnittlicher_abstand = 0

    print(f"Kleinster Abstand: {kleinster_abstand:.2f} m")
    print(f"Durchschnittlicher Abstand: {durchschnittlicher_abstand:.2f} m")

    if kleinster_abstand < 0.30:
        print("Notstopp!")
    else:
        print("Weiterfahren")"""), language="python")
    final_pro_task_code_box
    return (final_pro_task_code_box,)


@app.cell(hide_code=True)
def _(contextlib, final_pro_task_code_box, io, mo, np, render_task_output):
    final_pro_task_code_box_output_buffer = io.StringIO()
    with contextlib.redirect_stdout(final_pro_task_code_box_output_buffer):
        try:
            exec(final_pro_task_code_box.value, {"np": np})
        except Exception as e:
            print(f"Execution Error: {e}")

    render_task_output(
        mo,
        final_pro_task_code_box_output_buffer.getvalue(),
        """Kleinster Abstand: 0.25 m
    Durchschnittlicher Abstand: 0.55 m
    Notstopp!""",
    )
    return


if __name__ == "__main__":
    app.run()
