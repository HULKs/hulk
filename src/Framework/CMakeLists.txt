target_sources(hulk PRIVATE
  "${CMAKE_CURRENT_SOURCE_DIR}/Database.cpp"
  "${CMAKE_CURRENT_SOURCE_DIR}/Database.hpp"
  "${CMAKE_CURRENT_SOURCE_DIR}/DataType.hpp"
  "${CMAKE_CURRENT_SOURCE_DIR}/DebugDatabase.cpp"
  "${CMAKE_CURRENT_SOURCE_DIR}/DebugDatabase.hpp"
  "${CMAKE_CURRENT_SOURCE_DIR}/Messaging.cpp"
  "${CMAKE_CURRENT_SOURCE_DIR}/Messaging.hpp"
  "${CMAKE_CURRENT_SOURCE_DIR}/Module.cpp"
  "${CMAKE_CURRENT_SOURCE_DIR}/Module.hpp"
  "${CMAKE_CURRENT_SOURCE_DIR}/ModuleManagerInterface.cpp"
  "${CMAKE_CURRENT_SOURCE_DIR}/ModuleManagerInterface.hpp"
  "${CMAKE_CURRENT_SOURCE_DIR}/SharedObject.cpp"
  "${CMAKE_CURRENT_SOURCE_DIR}/SharedObject.hpp"
  "${CMAKE_CURRENT_SOURCE_DIR}/SharedObjectManager.cpp"
  "${CMAKE_CURRENT_SOURCE_DIR}/SharedObjectManager.hpp"
  "${CMAKE_CURRENT_SOURCE_DIR}/Thread.cpp"
  "${CMAKE_CURRENT_SOURCE_DIR}/Thread.hpp"
  "${CMAKE_CURRENT_SOURCE_DIR}/tuhh.cpp"
  "${CMAKE_CURRENT_SOURCE_DIR}/tuhh.hpp"
)

add_subdirectory("${CMAKE_CURRENT_LIST_DIR}/Configuration/")
add_subdirectory("${CMAKE_CURRENT_LIST_DIR}/Debug/")
add_subdirectory("${CMAKE_CURRENT_LIST_DIR}/Log/")
