# .ycm_extra_conf.py for nvim source code.
import os
import ycm_core

def DirectoryOfThisScript():
    return os.path.dirname(os.path.abspath(__file__))

def GetDatabase():
    with open(os.path.join(DirectoryOfThisScript(), '.current.tc')) as f:
        current_target = f.read()[:-1]

    with open(os.path.join(DirectoryOfThisScript(), '.current.bt')) as f:
        current_build_type = f.read()[:-1]

    compilation_database_folder = os.path.join(DirectoryOfThisScript(), 'build/' + current_target + '/' + current_build_type)
    if os.path.exists(compilation_database_folder):
        return ycm_core.CompilationDatabase(compilation_database_folder)
    return None


def IsHeaderFile(filename):
    extension = os.path.splitext(filename)[1]
    return extension in ['.h', '.hpp']


def GetCompilationInfoForFile(filename):
    database = GetDatabase()
    if not database:
        return None
    if IsHeaderFile(filename):
        basename = os.path.splitext(filename)[0]
        cpp_file = basename + '.cpp'
        # for pure headers (no cpp file), default to print.cpp as it exists in all four targets
        if not os.path.exists(cpp_file):
            normpath = os.path.normpath(cpp_file)
            sub_dirs = normpath[len(DirectoryOfThisScript())+1:].split(os.sep)
            cpp_file = os.path.join(DirectoryOfThisScript(), sub_dirs[0], sub_dirs[1], 'print.cpp')
            if not os.path.exists(cpp_file):
                return None
        compilation_info = database.GetCompilationInfoForFile(cpp_file)
        if compilation_info.compiler_flags_:
            return compilation_info
        return None
    return database.GetCompilationInfoForFile(filename)


def FlagsForFile(filename, **kwargs):
    compilation_info = GetCompilationInfoForFile(filename)
    if not compilation_info:
        return None
    # Add flags not needed for clang-the-binary,
    # but needed for libclang-the-library (YCM uses this last one).
    flags = (list(compilation_info.compiler_flags_)
             if compilation_info.compiler_flags_
             else [])
    extra_flags = ['-Wno-newline-eof']
    final_flags = flags + extra_flags
    return {
        'flags': final_flags,
        'do_cache': True
    }
