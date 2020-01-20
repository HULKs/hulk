#pragma once

#include "Framework/Module.hpp"

#include "Data/ImageData.hpp"
#include "Data/IntegralImageData.hpp"


class Brain;

class IntegralImageProvider : public Module<IntegralImageProvider, Brain>
{
public:
  /// the name of this module
  ModuleName name = "IntegralImageProvider";
  /**
   * IntegralImageProvider constructor
   * @param manager a reference to the brain object
   */
  explicit IntegralImageProvider(const ModuleManagerInterface& manager);
  void cycle() override;

private:
  /// The modes the integral image can be build
  enum class Mode
  {
    CB,
    CR,
    GREEN,
    GREEN_CHROMATICITY
  };

  /// a reference to the current image
  const Dependency<ImageData> imageData_;

  /// the down scale factor of the resulting integral image
  const Parameter<int> scale_;
  /// the mode in which the integral image is build
  const Parameter<int> mode_;

  template<Mode mode>
  inline unsigned int getValue(int x, int y) const;

  /*
   * @brief constructs the integral image in integralImageData_ based on the given mode
   * @tparam mode the mode in which the integral image should be build
   */
  template <Mode mode>
  void constructIntegralImage();

  Production<IntegralImageData> integralImageData_;
};
