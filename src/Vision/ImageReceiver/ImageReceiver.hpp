#pragma once

#include "Data/CycleInfo.hpp"
#include "Data/ImageData.hpp"
#include "Framework/Module.hpp"
#include "Hardware/Definitions.hpp"

class Brain;

class ImageReceiver : public Module<ImageReceiver, Brain>
{
public:
  /// the name of this module
  ModuleName name__{"ImageReceiver"};
  /**
   * @brief ImageReceiver gets camera handles from tuhhFramework and starts image capturing
   * @param manager a reference to the module manager
   * @author Arne Hasselbring
   */
  ImageReceiver(const ModuleManagerInterface& manager);
  /**
   * @brief ~ImageReceiver stops image capturing
   * @author Arne Hasselbring
   */
  ~ImageReceiver();
  /**
   * @brief cycle waits for the next image and stores it into the ImageData structure
   * @author Arne Hasselbring
   */
  void cycle() override;

private:
  /// some information about the cycle time
  Production<CycleInfo> cycleInfo_;
  /// the result of the ImageReceiver
  Production<ImageData> imageData_;
};
