include(vcpkg_common_functions)

vcpkg_from_github(
  OUT_SOURCE_PATH SOURCE_PATH
  REPO bhuman/CompiledNN
  REF f785096d243b6892f9f53dc188d3df8c9b601cb0
  SHA512 6b10b2509667a9a1b37d71748e64783240a019b81792bd141e0d576bca14fbd25ca8d3453f82a58147fc186c91cf318b0efcb0bdf84dc01c12d956fa77f6a26e
  HEAD_REF master
  PATCHES
    static-installing.patch
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
