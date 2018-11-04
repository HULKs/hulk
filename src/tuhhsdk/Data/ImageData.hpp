#pragma once

#include "Framework/DataType.hpp"
#include "Hardware/CameraInterface.hpp"
#include "Tools/Storage/Image.hpp"
#include "Tools/Storage/Image422.hpp"
#include "Tools/Time.hpp"
#include <Modules/Debug/Debug.h>

class ImageData : public DataType<ImageData>
{
public:
  /// the name of this DataType
  DataTypeName name = "ImageData";
  /// the camera that took the image
  Camera camera;
  /// a string identifying the camera
  std::string identification;
  /// the pixel data and size as 422 image
  Image422 image422;
  /// the system time at which the first pixel has been recorded
  TimePoint timestamp;
  /// the number of seconds that had to be waited for the image
  float wait_time;
  /// true if the image data is actually provided
  bool is_provided = false;
  /**
   * @brief reset sets the image to a defined state, does nothing at the moment
   */
  void reset()
  {
    // THIS MUST BE EMPTY FOR SECURTIY REASONS
  }

  virtual void toValue(Uni::Value& value) const
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    // TODO: May do something here.
  }

  virtual void fromValue(const Uni::Value&) {}
};
