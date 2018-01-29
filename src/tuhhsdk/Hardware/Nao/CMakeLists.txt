cmake_minimum_required(VERSION 3.5)
# functions that are used by the aldebaran cmake configs because they think
# the can use qibuild. They are not needed in our context, so they are
# implmented as noops.
function(export_header unused)
endfunction()
function(export_lib unused)
endfunction()
set(CMAKE_TOOLCHAIN_FILE "${TOOLCHAIN_DIR}/naoqi/cross-config.cmake")
set(CMAKE_PREFIX_PATH "${TOOLCHAIN_DIR}/naoqi/libnaoqi")
project(tuhhALModule)

set(SOURCES
  ../../Definitions/keys.cpp
  SMO.cpp
  ALModule/BatteryDisplay.cpp
  ALModule/DcmConnectorAL.cpp
  ALModule/libTUHH.cpp
)

set(HEADERS
  ../../Definitions/keys.h
  SMO.h
  ALModule/BatteryDisplay.hpp
  ALModule/DcmConnectorAL.h
  ALModule/DcmConverter.hpp
  ALModule/libTUHH.h
)

add_definitions(-DBOOST_PARAMETER_MAX_ARITY=7)
add_definitions(-DBOOST_SIGNALS_NO_DEPRECATION_WARNING)
add_definitions(-DBOOST_NO_DEFAULTED_FUNCTIONS)
set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} -std=gnu++0x -Wall -Wextra -pedantic")
set(CMAKE_CXX_FLAGS_DEBUG "-Werror -pedantic-errors -g")
set(CMAKE_CXX_FLAGS_DEVELOP "-Werror -pedantic-errors -O2")
set(CMAKE_CXX_FLAGS_RELEASE "-O2 -DNDEBUG")

add_library(${PROJECT_NAME} SHARED ${SOURCES} ${HEADERS})

target_include_directories(${PROJECT_NAME} PUBLIC
  ${CMAKE_CURRENT_SOURCE_DIR}/../../)

find_package(Boost REQUIRED)
find_package(Boost_system REQUIRED)
find_package(Boost_signals REQUIRED)
target_include_directories(${PROJECT_NAME} SYSTEM PUBLIC ${BOOST_INCLUDE_DIRS})
target_link_libraries(${PROJECT_NAME} ${BOOST_SYSTEM_LIBRARIES} ${BOOST_SIGNALS_LIBRARIES})

find_package(alcommon REQUIRED)
find_package(alproxies REQUIRED)
target_include_directories(${PROJECT_NAME} SYSTEM PUBLIC ${ALCOMMON_INCLUDE_DIRS} ${ALPROXIES_INCLUDE_DIRS})
target_link_libraries(${PROJECT_NAME} ${ALCOMMON_LIBRARIES} ${ALPROXIES_LIBRARIES})
