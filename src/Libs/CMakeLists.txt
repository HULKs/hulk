target_sources(hulk PRIVATE
  "${CMAKE_CURRENT_SOURCE_DIR}/jsoncpp.cpp"
)

add_subdirectory("${CMAKE_CURRENT_LIST_DIR}/AStarSearch/")
add_subdirectory("${CMAKE_CURRENT_LIST_DIR}/json/")
