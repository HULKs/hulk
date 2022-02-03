#include "extract.hpp"

#include <fstream>

namespace Hulks::GridCropper
{

#include "classifier_contents.hpp"

  std::filesystem::path extractClassifier()
  {
    auto classifierPath = std::filesystem::temp_directory_path() / "grid-cropper-classifier.hdf5";

    std::ofstream classifierFile{classifierPath, std::ios_base::out | std::ios_base::trunc};
    classifierFile.write(classifier, CLASSIFIER_SIZE);

    return classifierPath;
  }

#include "positioner_contents.hpp"

  std::filesystem::path extractPositioner()
  {
    auto positionerPath = std::filesystem::temp_directory_path() / "grid-cropper-positioner.hdf5";

    std::ofstream positionerFile{positionerPath, std::ios_base::out | std::ios_base::trunc};
    positionerFile.write(positioner, POSITIONER_SIZE);

    return positionerPath;
  }

} // namespace Hulks::GridCropper
