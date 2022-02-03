#pragma once

#include "Data/StrikerAction.hpp"
#include "Framework/DataType.hpp"

class PenaltyStrikerAction : public DataType<PenaltyStrikerAction, StrikerAction>
{
public:
  /// the name of this DataType
  DataTypeName name__{"PenaltyStrikerAction"};
};
