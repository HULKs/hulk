import argparse
import platform
import shutil
import subprocess
from pathlib import Path


def main() -> int:
    parser = argparse.ArgumentParser(description="Open the K1 MuJoCo viewer.")
    parser.add_argument(
        "--local",
        action="store_true",
        help="Use the current Python runtime instead of Windows on WSL.",
    )
    args = parser.parse_args()
    script = Path(__file__).resolve()

    if not args.local and "microsoft" in platform.release().lower():
        uv = shutil.which("uv.exe")
        wslpath = shutil.which("wslpath")
        if uv is None or wslpath is None:
            parser.error(
                "WSL GPU rendering needs Windows uv.exe and wslpath on PATH. "
                "Install Windows uv, or pass --local to use the WSL renderer."
            )
        windows_script = subprocess.check_output(  # noqa: S603
            [wslpath, "-w", str(script)], text=True
        ).strip()
        print("Opening the K1 viewer with native Windows OpenGL.", flush=True)
        return subprocess.run(  # noqa: S603
            [
                uv,
                "run",
                "--no-project",
                "--python",
                "3.12",
                "--with",
                "mujoco==3.3.6",
                "python",
                windows_script,
                "--local",
            ],
            cwd=script.parent,
            check=False,
        ).returncode

    from mujoco import MjData, MjModel
    from mujoco.viewer import launch

    model = MjModel.from_xml_path(str(script.parent / "K1" / "K1.xml"))
    data = MjData(model)
    launch(model, data)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
