target_sources(hulk PRIVATE
  "${CMAKE_CURRENT_SOURCE_DIR}/Color.hpp"
  "${CMAKE_CURRENT_SOURCE_DIR}/EnumArray.hpp"
  "${CMAKE_CURRENT_SOURCE_DIR}/Image.cpp"
  "${CMAKE_CURRENT_SOURCE_DIR}/Image.hpp"
  "${CMAKE_CURRENT_SOURCE_DIR}/Image422.cpp"
  "${CMAKE_CURRENT_SOURCE_DIR}/Image422.hpp"
)

add_subdirectory("${CMAKE_CURRENT_LIST_DIR}/UniValue/")
add_subdirectory("${CMAKE_CURRENT_LIST_DIR}/XPM/")
