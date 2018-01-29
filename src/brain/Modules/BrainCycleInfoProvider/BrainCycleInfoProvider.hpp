#pragma once

#include "Data/CycleInfo.hpp"
#include "Data/ImageData.hpp"
#include "Framework/Module.hpp"


class Brain;

class BrainCycleInfoProvider : public Module<BrainCycleInfoProvider, Brain>
{
public:
  /**
   * @brief BrainCycleInfoProvider initializes members
   * @param manager a reference to brain
   */
  BrainCycleInfoProvider(const ModuleManagerInterface& manager);
  /**
   * @brief cycle updates the cycle info
   */
  void cycle();

private:
  /// a reference to the image data
  const Dependency<ImageData> imageData_;
  /// a reference to the cycle info
  Production<CycleInfo> cycleInfo_;
};
