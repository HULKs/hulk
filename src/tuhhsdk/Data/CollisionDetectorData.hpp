#pragma once

#include "Framework/DataType.hpp"

class CollisionDetectorData : public DataType<CollisionDetectorData>
{
public:
  /// the name of this DataType
  DataTypeName name = "CollisionDetectorData";

  // These fields hold the current state of the prediction/detection
  bool collisionLeft;
  bool collisionRight;
  bool duel;
  // These fields hold the previous true state for CollisionHandler.json -> timeHoldState": 2,
  // needed for some dependant modules, like the remove arms stuff.
  bool collisionLeftRigid;
  bool collisionRightRigid;
  bool duelRigid;

  void reset() override
  {
    collisionLeft = false;
    collisionRight = false;
    duel = false;

    collisionLeftRigid = false;
    collisionRightRigid = false;
    duelRigid = false;
  }

  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["collisionLeft"] << collisionLeft;
    value["collisionRight"] << collisionRight;
    value["duel"] << duel;
    value["collisionLeftRigid"] << collisionLeftRigid;
    value["collisionRightRigid"] << collisionRightRigid;
    value["duelRigid"] << duelRigid;
  }

  void fromValue(const Uni::Value& value) override
  {
    value["collisionLeft"] >> collisionLeft;
    value["collisionRight"] >> collisionRight;
    value["duel"] >> duel;
    value["collisionLeftRigid"] >> collisionLeftRigid;
    value["collisionRightRigid"] >> collisionRightRigid;
    value["duelRigid"] >> duelRigid;
  }
};
