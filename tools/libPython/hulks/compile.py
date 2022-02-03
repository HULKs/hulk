import hulks
from pathlib import Path
import subprocess
import os


class CompilationError(Exception):
    pass


def compile(target: str, build_type: str, is_verbose: bool):
    hulks.logger.info(
        f'Compiling for target: {target} build_type: {build_type}')
    hulks.force_symlink(target,
                        hulks.PROJECT_ROOT / 'build/current-target',
                        target_is_directory=True)
    hulks.force_symlink(build_type,
                        hulks.PROJECT_ROOT /
                        'build/current-target/current-buildtype',
                        target_is_directory=True)
    build_dir = hulks.PROJECT_ROOT / 'build/current-target/current-buildtype/'
    cmake_call = f"cmake --build {build_dir} {'-v' if is_verbose else ''} --config {build_type}"
    environment_setup = hulks.PROJECT_ROOT / Path(
        'sdk/current/environment-setup-corei7-64-aldebaran-linux')
    command = ''
    if target == 'NAO':
        command += (
            f'. {environment_setup};'
            'export CC=$CLANGCC;'
            'export CXX=$CLANGCXX;'
        )
    command += cmake_call
    try:
        subprocess.run(command, shell=True, check=True)
    except subprocess.CalledProcessError:
        raise CompilationError()
    if target == "NAO" and (not os.path.exists(build_dir / 'hulk.debug')
                            or os.path.getctime(build_dir / 'hulk.debug') <
                            os.path.getctime(build_dir / 'hulk')):
        hulks.logger.info("Splitting debug information from executable")
        debug_split_command = (
            f'objcopy --only-keep-debug --compress-debug-sections {build_dir / "hulk"} {build_dir / "hulk.debug"};'
            f'strip {build_dir / "hulk"};'
            f'objcopy --add-gnu-debuglink="{build_dir / "hulk.debug"}" {build_dir / "hulk"};'
            f'touch {build_dir / "hulk.debug"}'
        )
        subprocess.run(debug_split_command, shell=True, check=True)
