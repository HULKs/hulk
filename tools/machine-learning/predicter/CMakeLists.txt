cmake_minimum_required(VERSION 3.10)

project(predicter LANGUAGES CXX)

find_program(CCACHE_PROGRAM ccache)
if(CCACHE_PROGRAM)
  set(CMAKE_CXX_COMPILER_LAUNCHER "${CCACHE_PROGRAM}")
  set(CMAKE_CUDA_COMPILER_LAUNCHER "${CCACHE_PROGRAM}") # CMake 3.9+
endif()

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_EXPORT_COMPILE_COMMANDS ON)

find_package(hdf5 CONFIG REQUIRED)
find_package(protobuf CONFIG REQUIRED)
find_package(CompiledNN CONFIG REQUIRED)
find_package(cxxopts CONFIG REQUIRED)
find_package(nlohmann_json CONFIG REQUIRED)

set(TARGET_NAME predicter)
add_executable(${TARGET_NAME}
  "${CMAKE_CURRENT_SOURCE_DIR}/main.cpp"
)

target_include_directories(${TARGET_NAME} PRIVATE
  "${CMAKE_CURRENT_SOURCE_DIR}/"
)

target_link_libraries(${TARGET_NAME} PRIVATE
  CompiledNN::CompiledNN
  cxxopts::cxxopts
  nlohmann_json::nlohmann_json
)

install(TARGETS ${TARGET_NAME})
