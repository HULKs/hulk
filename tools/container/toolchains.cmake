set(CMAKE_SYSTEM_NAME Linux)
set(CMAKE_SYSTEM_PROCESSOR aarch64)

# # Force CMake to use the host's make and git, not the target's
# set(CMAKE_MAKE_PROGRAM "/usr/bin/make" CACHE FILEPATH "Path to the make program" FORCE)
# set(GIT_EXECUTABLE "/usr/bin/git" CACHE FILEPATH "Path to the git program" FORCE)
# set(GIT "/usr/bin/git" CACHE FILEPATH "Path to git" FORCE)
# set(PKG_CONFIG_EXECUTABLE "/usr/bin/pkg-config" CACHE FILEPATH "Path to the pkg-config program" FORCE)

# # Set the cross compiler paths based on Nvidia's documentation
# set(TOOLCHAIN_PATH /l4t/toolchain/aarch64--glibc--stable-2022.08-1)
# set(CROSS_COMPILE_PREFIX aarch64-buildroot-linux-gnu)
# set(THIRD_PARTY_DIR /workspace/third_party)

# # Force use of the cross-compilers
# set(CMAKE_C_COMPILER ${TOOLCHAIN_PATH}/bin/${CROSS_COMPILE_PREFIX}-gcc)
# set(CMAKE_CXX_COMPILER ${TOOLCHAIN_PATH}/bin/${CROSS_COMPILE_PREFIX}-g++)
# set(CMAKE_ASM_COMPILER ${TOOLCHAIN_PATH}/bin/${CROSS_COMPILE_PREFIX}-gcc)

# # Configuration for linker, ar and ranlib
# set(CMAKE_AR ${TOOLCHAIN_PATH}/bin/${CROSS_COMPILE_PREFIX}-ar)
# set(CMAKE_RANLIB ${TOOLCHAIN_PATH}/bin/${CROSS_COMPILE_PREFIX}-ranlib)
# set(CMAKE_LINKER ${TOOLCHAIN_PATH}/bin/${CROSS_COMPILE_PREFIX}-ld)

# set(CMAKE_CUDA_COMPILER /usr/local/cuda/bin/nvcc)
# set(CMAKE_CUDA_FLAGS "-ccbin=${CMAKE_CXX_COMPILER}")

# #set(CUDA_TOOLKIT_ROOT_DIR /usr/local/cuda/)

# # Set the sysroot and include paths
# set(COMPILER_SYSROOT ${TOOLCHAIN_PATH}/${CROSS_COMPILE_PREFIX}/sysroot)
# set(TARGET_FS /l4t/targetfs)

# # Add compiler and linker search paths
# set(TARGET_LIB_PATH ${TARGET_FS}/usr/lib)
# set(TARGET_ARCH_LIB_PATH ${TARGET_FS}/usr/lib/aarch64-linux-gnu)
# set(COMPILER_LIB_PATH ${COMPILER_SYSROOT}/usr/lib)

# # Explicitly set include and library paths for find_* commands
# set(CMAKE_INCLUDE_PATH ${TARGET_FS}/usr/include ${TARGET_FS}/usr/include/aarch64-linux-gnu ${COMPILER_SYSROOT}/usr/include)
# set(CMAKE_LIBRARY_PATH ${TARGET_ARCH_LIB_PATH} ${TARGET_LIB_PATH} ${COMPILER_LIB_PATH})

# # Set flags to override the compiler's default include directories
# set(INCLUDE_FLAGS "\
# -isystem ${TOOLCHAIN_PATH}/${CROSS_COMPILE_PREFIX}/include/c++/11.3.0 \
# -isystem ${TOOLCHAIN_PATH}/${CROSS_COMPILE_PREFIX}/include/c++/11.3.0/${CROSS_COMPILE_PREFIX} \
# -isystem ${TOOLCHAIN_PATH}/${CROSS_COMPILE_PREFIX}/include/c++/11.3.0/backward \
# -isystem ${TOOLCHAIN_PATH}/lib/gcc/${CROSS_COMPILE_PREFIX}/11.3.0/include \
# -isystem ${COMPILER_SYSROOT}/usr/include \
# -isystem ${TARGET_FS}/usr/include \
# -isystem ${TARGET_FS}/usr/include/aarch64-linux-gnu \
# ")

# # Set linker flags to include all necessary startup files and library paths
# # The order of the library paths is important
# set(LINKER_FLAGS "\
# --sysroot=${TARGET_FS} \
# -L${TARGET_ARCH_LIB_PATH} \
# -L${TARGET_LIB_PATH} \
# -L${COMPILER_LIB_PATH} \
# -L${THIRD_PARTY_DIR}/lib \
# -L/usr/local/cuda-12.6/targets/aarch64-linux/lib \
# -B${TARGET_ARCH_LIB_PATH} \
# -B${TARGET_LIB_PATH} \
# -B${COMPILER_LIB_PATH} \
# -Wl,-rpath-link,${TARGET_ARCH_LIB_PATH} \
# -Wl,-rpath-link,${TARGET_LIB_PATH} \
# -Wl,--allow-shlib-undefined \
# ")

# # Set compiler and linker flags
# set(CMAKE_C_FLAGS "--sysroot=${TARGET_FS} ${INCLUDE_FLAGS}" CACHE STRING "" FORCE)
# set(CMAKE_CXX_FLAGS "--sysroot=${TARGET_FS} ${INCLUDE_FLAGS}" CACHE STRING "" FORCE)
# set(CMAKE_EXE_LINKER_FLAGS "${LINKER_FLAGS}" CACHE STRING "" FORCE)
# set(CMAKE_SHARED_LINKER_FLAGS "${LINKER_FLAGS}" CACHE STRING "" FORCE)

# # --- RPATH Handling ---
# # Don't add build paths to RPATH
# set(CMAKE_SKIP_BUILD_RPATH TRUE CACHE BOOL "Skip RPATH for build tree" FORCE)
# # Use the RPATH intended for installation even in the build tree
# set(CMAKE_BUILD_WITH_INSTALL_RPATH TRUE CACHE BOOL "Build with install RPATH" FORCE)
# # Set the RPATH relative to the executable's location for bundled libraries
# set(CMAKE_INSTALL_RPATH "\$ORIGIN/../lib" CACHE STRING "Installation RPATH" FORCE)
# # --- End RPATH Handling ---

# # Prevent CMake from trying compiler tests that won't work in cross-compilation
# set(CMAKE_TRY_COMPILE_TARGET_TYPE STATIC_LIBRARY)

# # Only search for libraries and includes in the target directories
# set(CMAKE_FIND_ROOT_PATH
#     ${TARGET_FS}
#     ${TARGET_ARCH_LIB_PATH}
#     ${TARGET_LIB_PATH}
#     ${COMPILER_SYSROOT}
#     ${TOOLCHAIN_PATH}
#     /usr/local/cuda-12.6
#     /usr/local/cuda-12.6/targets/aarch64-linux
# )
# set(CMAKE_FIND_ROOT_PATH_MODE_PROGRAM NEVER)
# set(CMAKE_FIND_ROOT_PATH_MODE_LIBRARY ONLY)
# set(CMAKE_FIND_ROOT_PATH_MODE_INCLUDE ONLY)
# set(CMAKE_FIND_ROOT_PATH_MODE_PACKAGE ONLY)

# # Explicitly set the prefix path for find_package within the sysroot
# set(CMAKE_PREFIX_PATH ${TARGET_FS}/usr)

# # Add target library path for find_library
# link_directories(${TARGET_ARCH_LIB_PATH})

SET(CMAKE_SYSTEM_VERSION 1)
SET(CMAKE_C_COMPILER aarch64-linux-gnu-gcc)
SET(CMAKE_CXX_COMPILER aarch64-linux-gnu-g++)
SET(CMAKE_FIND_ROOT_PATH_MODE_PROGRAM NEVER)
SET(CMAKE_FIND_ROOT_PATH_MODE_LIBRARY ONLY)
SET(CMAKE_FIND_ROOT_PATH_MODE_INCLUDE ONLY)
SET(CMAKE_FIND_ROOT_PATH_MODE_PACKAGE ONLY)
SET(CMAKE_FIND_ROOT_PATH /l4t/targetfs)
