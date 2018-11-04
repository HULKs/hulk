#pragma once

#include "Framework/DataType.hpp"
#include "Modules/Configuration/Configuration.h"

/**
 * @brief FakeImageData The fake image data is a dummy data type used to synchronize the threads
 * with the incoming data stream, if there is no camera image to sync it with
 */
class FakeImageData : public DataType<FakeImageData>
{
public:
  /// the name of this DataType
  DataTypeName name = "FakeImageData";
  /// the image size of the faked image.
  Vector2i imageSize = {640, 480};

  void reset() {}

  virtual void toValue(Uni::Value&) const {}

  virtual void fromValue(const Uni::Value&) {}
};
