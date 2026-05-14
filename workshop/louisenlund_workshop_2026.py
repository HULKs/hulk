import marimo

__generated_with = "0.23.6"
app = marimo.App(width="medium")


@app.cell
def _():
    import marimo as mo
    import workshop

    return mo, workshop


@app.cell
def _(mo, workshop):
    mo.md(f"""
    Version {workshop.TEST}
    """)
    return


if __name__ == "__main__":
    app.run()
