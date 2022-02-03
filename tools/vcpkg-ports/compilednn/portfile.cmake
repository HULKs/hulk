vcpkg_from_github(
  OUT_SOURCE_PATH SOURCE_PATH
  REPO bhuman/CompiledNN
  REF 60592207e8d17e27e6c2384a8e92a5e023637432
  SHA512 a0f953767789fd6f7c6412265a6eec4ca2e372ee6bb7a8bacbd22b3642888210643e71ed34f3728c2e2fa99556be04f2c1c53471d311cb9724223223d0e0a894
  HEAD_REF master
  PATCHES
    cmake-3.14-static.patch
)

vcpkg_configure_cmake(
  SOURCE_PATH ${SOURCE_PATH}
  PREFER_NINJA
)
vcpkg_install_cmake()
vcpkg_fixup_cmake_targets(CONFIG_PATH share/cmake/compilednn)

# remove duplicate include files
file(REMOVE_RECURSE ${CURRENT_PACKAGES_DIR}/debug/include)

# install copyright in respect to vcpkg convention
file(INSTALL ${SOURCE_PATH}/LICENSE DESTINATION ${CURRENT_PACKAGES_DIR}/share/compilednn RENAME copyright)
