target_sources(hulk PRIVATE
  "${CMAKE_CURRENT_SOURCE_DIR}/NotPenalized.cpp"
  "${CMAKE_CURRENT_SOURCE_DIR}/RootBehavior.cpp"
)

add_subdirectory("${CMAKE_CURRENT_LIST_DIR}/GameStates/")
add_subdirectory("${CMAKE_CURRENT_LIST_DIR}/Head/")
add_subdirectory("${CMAKE_CURRENT_LIST_DIR}/Roles/")
add_subdirectory("${CMAKE_CURRENT_LIST_DIR}/Skills/")
