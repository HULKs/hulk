#pragma once

#include "Data/KeeperAction.hpp"
#include "Framework/DataType.hpp"
#include "Tools/Math/Pose.hpp"


class ReplacementKeeperAction : public DataType<ReplacementKeeperAction>
{
public:
  /// the name of this DataType
  DataTypeName name = "ReplacementKeeperAction";

  // sum all actions the replacement keeper is allowed to perform
  unsigned int permission = static_cast<unsigned int>(KeeperAction::Type::BLOCK_GOAL);

  /// the action to be performed by the replacement keeper
  KeeperAction::Action action;

  void reset() override
  {
    action = KeeperAction::Action();
  }

  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["permission"] << permission;
    value["action"] << action;
  }
  void fromValue(const Uni::Value& value) override
  {
    value["permission"] >> permission;
    value["action"] >> action;
  }
};
