include(vcpkg_common_functions)

# WINDOWS_EXPORT_ALL_SYMBOLS doesn't work.
# unresolved external symbol "public: static unsigned int const foonathan::memory::detail::memory_block_stack::implementation_offset
vcpkg_check_linkage(ONLY_STATIC_LIBRARY)

vcpkg_from_github(
    OUT_SOURCE_PATH SOURCE_PATH
    REPO foonathan/memory
    REF c4345991bf96b2f79b54d8e3fa60acaab169af6d
    SHA512 99f8e46b1e3d5a86e7db8a43da0d37a2d4b9518e752ee4d7d867856c3ef280e0d060f12703c4275f84eda609695a698a8f051d81f54ba2e515903fe45b7e17a2
    HEAD_REF master
    PATCHES
      vcpkg.patch
)

vcpkg_check_features(OUT_FEATURE_OPTIONS FEATURE_OPTIONS
    tool FOONATHAN_MEMORY_BUILD_TOOLS
)

vcpkg_configure_cmake(
    SOURCE_PATH ${SOURCE_PATH}
    PREFER_NINJA
    OPTIONS
        ${FEATURE_OPTIONS}
        -DFOONATHAN_MEMORY_BUILD_EXAMPLES=OFF
        -DFOONATHAN_MEMORY_BUILD_TESTS=OFF
)

vcpkg_install_cmake()
vcpkg_fixup_cmake_targets(CONFIG_PATH share/foonathan_memory TARGET_PATH share/foonathan_memory)
vcpkg_copy_pdbs()

# Place header files into the right folders
# The original layout is not a problem for CMake-based project.
file(COPY
    ${CURRENT_PACKAGES_DIR}/include/foonathan_memory/foonathan
    DESTINATION ${CURRENT_PACKAGES_DIR}/include
)
file(COPY
    ${CURRENT_PACKAGES_DIR}/debug/include/foonathan_memory/config_impl_dbg.hpp
    DESTINATION ${CURRENT_PACKAGES_DIR}/include/foonathan/memory
)
file(COPY
    ${CURRENT_PACKAGES_DIR}/include/foonathan_memory/config_impl_rel.hpp
    DESTINATION ${CURRENT_PACKAGES_DIR}/include/foonathan/memory
)
file(REMOVE_RECURSE
    ${CURRENT_PACKAGES_DIR}/include/foonathan_memory
)
vcpkg_replace_string(
    ${CURRENT_PACKAGES_DIR}/share/foonathan_memory/foonathan_memory-config.cmake
    "\${_IMPORT_PREFIX}/include/foonathan_memory"
    "\${_IMPORT_PREFIX}/include"
)
# Place header files into the right folders - Done!

# The Debug version of this lib is built with:
# #define FOONATHAN_MEMORY_DEBUG_FILL 1
# and Release version is built with:
# #define FOONATHAN_MEMORY_DEBUG_FILL 0
# We only have the Release version header files installed, however.
vcpkg_replace_string(
    ${CURRENT_PACKAGES_DIR}/include/foonathan/memory/detail/debug_helpers.hpp
    "#if FOONATHAN_MEMORY_DEBUG_FILL"
    "#ifndef NDEBUG //#if FOONATHAN_MEMORY_DEBUG_FILL"
)

file(REMOVE_RECURSE
    ${CURRENT_PACKAGES_DIR}/debug/include
    ${CURRENT_PACKAGES_DIR}/debug/share
)

file(REMOVE
    ${CURRENT_PACKAGES_DIR}/debug/LICENSE
    ${CURRENT_PACKAGES_DIR}/debug/README.md
    ${CURRENT_PACKAGES_DIR}/share/LICENSE
    ${CURRENT_PACKAGES_DIR}/share/README.md
    ${CURRENT_PACKAGES_DIR}/LICENSE
    ${CURRENT_PACKAGES_DIR}/README.md
)

if(NOT VCPKG_CMAKE_SYSTEM_NAME OR
   VCPKG_CMAKE_SYSTEM_NAME STREQUAL "WindowsStore")
    set(EXECUTABLE_SUFFIX ".exe")
else()
    set(EXECUTABLE_SUFFIX "")
endif()

if(EXISTS ${CURRENT_PACKAGES_DIR}/bin/nodesize_dbg${EXECUTABLE_SUFFIX})
    file(COPY
        ${CURRENT_PACKAGES_DIR}/bin/nodesize_dbg${EXECUTABLE_SUFFIX}
        DESTINATION ${CURRENT_PACKAGES_DIR}/tools/${PORT}
    )
    vcpkg_copy_tool_dependencies(${CURRENT_PACKAGES_DIR}/tools/${PORT})

    if(VCPKG_LIBRARY_LINKAGE STREQUAL static)
        file(REMOVE_RECURSE
            ${CURRENT_PACKAGES_DIR}/bin
            ${CURRENT_PACKAGES_DIR}/debug/bin
        )
    else()
        file(REMOVE
            ${CURRENT_PACKAGES_DIR}/bin/nodesize_dbg${EXECUTABLE_SUFFIX}
            ${CURRENT_PACKAGES_DIR}/debug/bin/nodesize_dbg${EXECUTABLE_SUFFIX}
        )
    endif()
endif()

# Handle copyright
configure_file(${SOURCE_PATH}/LICENSE ${CURRENT_PACKAGES_DIR}/share/${PORT}/copyright COPYONLY)

# CMake integration test
vcpkg_test_cmake(PACKAGE_NAME foonathan_memory)
