#pragma once

#include "Data/CycleInfo.hpp"
#include "Data/ImageData.hpp"
#include "Data/LabelData.hpp"
#include "Framework/Module.hpp"


class Brain;

/**
 * @brief The LabelProvider class
 */
class LabelProvider : public Module<LabelProvider, Brain>
{
public:
  /// the name of this module
  ModuleName name__{"LabelProvider"};
  /**
   * @brief Provides data from the labeling tool annotate.
   * @param manager the module manager interface
   * @author Georg Felbinger
   */
  LabelProvider(const ModuleManagerInterface& manager);

  void cycle();

private:
  /// for calculating pixel coordinates from normalized ones
  const Dependency<ImageData> imageData_;

  Production<LabelData> labelData_;
};
