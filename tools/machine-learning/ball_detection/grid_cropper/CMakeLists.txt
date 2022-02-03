find_package(hdf5 CONFIG REQUIRED)
find_package(protobuf CONFIG REQUIRED)
find_package(CompiledNN CONFIG REQUIRED)
find_package(cxxopts CONFIG REQUIRED)
find_path(STB_INCLUDE_DIR "stb_image.h")
find_package(nlohmann_json CONFIG REQUIRED)
find_package(Python COMPONENTS Interpreter REQUIRED)

set(TARGET_NAME grid-cropper)
add_custom_command(
  COMMAND "${Python_EXECUTABLE}" "${CMAKE_CURRENT_SOURCE_DIR}/neuralnetworks/encode.py" "${CMAKE_CURRENT_SOURCE_DIR}/neuralnetworks/classifier.hdf5" "classifier" "${CMAKE_CURRENT_SOURCE_DIR}/neuralnetworks/classifier_contents.hpp"
  DEPENDS "${CMAKE_CURRENT_SOURCE_DIR}/neuralnetworks/encode.py" "${CMAKE_CURRENT_SOURCE_DIR}/neuralnetworks/classifier.hdf5"
  OUTPUT "${CMAKE_CURRENT_SOURCE_DIR}/neuralnetworks/classifier_contents.hpp"
  COMMENT "Converting classifier neural network into binary data."
)

add_custom_command(
  COMMAND "${Python_EXECUTABLE}" "${CMAKE_CURRENT_SOURCE_DIR}/neuralnetworks/encode.py" "${CMAKE_CURRENT_SOURCE_DIR}/neuralnetworks/positioner.hdf5" "positioner" "${CMAKE_CURRENT_SOURCE_DIR}/neuralnetworks/positioner_contents.hpp"
  DEPENDS "${CMAKE_CURRENT_SOURCE_DIR}/neuralnetworks/encode.py" "${CMAKE_CURRENT_SOURCE_DIR}/neuralnetworks/positioner.hdf5"
  OUTPUT "${CMAKE_CURRENT_SOURCE_DIR}/neuralnetworks/positioner_contents.hpp"
  COMMENT "Converting positioner neural network into binary data."
)

add_executable(${TARGET_NAME}
  "${CMAKE_CURRENT_SOURCE_DIR}/Circle.hpp"
  "${CMAKE_CURRENT_SOURCE_DIR}/CircleWithMetadata.hpp"
  "${CMAKE_CURRENT_SOURCE_DIR}/neuralnetworks/classifier_contents.hpp"
  "${CMAKE_CURRENT_SOURCE_DIR}/neuralnetworks/extract.cpp"
  "${CMAKE_CURRENT_SOURCE_DIR}/neuralnetworks/extract.hpp"
  "${CMAKE_CURRENT_SOURCE_DIR}/neuralnetworks/positioner_contents.hpp"
  "${CMAKE_CURRENT_SOURCE_DIR}/Cluster.hpp"
  "${CMAKE_CURRENT_SOURCE_DIR}/Configuration.hpp"
  "${CMAKE_CURRENT_SOURCE_DIR}/EvaluationResult.hpp"
  "${CMAKE_CURRENT_SOURCE_DIR}/Image.hpp"
  "${CMAKE_CURRENT_SOURCE_DIR}/main.cpp"
  "${CMAKE_CURRENT_SOURCE_DIR}/Processor.cpp"
  "${CMAKE_CURRENT_SOURCE_DIR}/Processor.hpp"
)

target_include_directories(${TARGET_NAME} PRIVATE
  "${CMAKE_CURRENT_SOURCE_DIR}/"
  "${CMAKE_CURRENT_SOURCE_DIR}/neuralnetworks/"
  "${STB_INCLUDE_DIR}"
)

target_link_libraries(${TARGET_NAME} PRIVATE
  CompiledNN::CompiledNN
  cxxopts::cxxopts
  nlohmann_json::nlohmann_json
  runner
)

install(TARGETS ${TARGET_NAME})
