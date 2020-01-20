#pragma once

#include <Data/MotionOutput.hpp>

class SitDownOutput : public DataType<SitDownOutput, MotionOutput>
{
public:
  /// the name of this DataType
  DataTypeName name = "SitDownOutput";

  bool isSitting = false;
};
