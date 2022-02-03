#pragma once

#include "Data/StrikerAction.hpp"
#include "Framework/DataType.hpp"

class SetPlayStrikerAction : public DataType<SetPlayStrikerAction, StrikerAction>
{
public:
  /// the name of this DataType
  DataTypeName name__{"SetPlayStrikerAction"};
};
