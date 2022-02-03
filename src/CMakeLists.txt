target_include_directories(hulk PRIVATE
  "${CMAKE_CURRENT_SOURCE_DIR}/"
)

add_subdirectory("${CMAKE_CURRENT_LIST_DIR}/Brain/")
add_subdirectory("${CMAKE_CURRENT_LIST_DIR}/Data/")
add_subdirectory("${CMAKE_CURRENT_LIST_DIR}/Framework/")
add_subdirectory("${CMAKE_CURRENT_LIST_DIR}/Hardware/")
add_subdirectory("${CMAKE_CURRENT_LIST_DIR}/Libs/")
add_subdirectory("${CMAKE_CURRENT_LIST_DIR}/Messages/")
add_subdirectory("${CMAKE_CURRENT_LIST_DIR}/Motion/")
add_subdirectory("${CMAKE_CURRENT_LIST_DIR}/Tools/")
add_subdirectory("${CMAKE_CURRENT_LIST_DIR}/Vision/")
