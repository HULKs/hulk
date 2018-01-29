#pragma once

#include "Framework/DataType.hpp"


class WorldState : public DataType<WorldState>
{
public:
  bool ballInOwnHalf;
  bool ballInLeftHalf;
  bool robotInOwnHalf;
  bool robotInLeftHalf;
  bool ballValid;
  bool robotValid;
  bool ballIsFree;

  void reset()
  {
    ballValid = false;
    robotValid = false;
    ballIsFree = false;
  }

  virtual void toValue(Uni::Value& value) const
  {
    value["ballInOwnHalf"] << ballInOwnHalf;
    value["ballInLeftHalf"] << ballInLeftHalf;
    value["robotInOwnHalf"] << robotInOwnHalf;
    value["robotInLeftHalf"] << robotInLeftHalf;
    value["ballValid"] << ballValid;
    value["robotValid"] << robotValid;
    value["ballIsFree"] << ballIsFree;
  }

  virtual void fromValue(const Uni::Value& value)
  {
    value["ballInOwnHalf"] >> ballInOwnHalf;
    value["ballInLeftHalf"] >> ballInLeftHalf;
    value["robotInOwnHalf"] >> robotInOwnHalf;
    value["robotInLeftHalf"] >> robotInLeftHalf;
    value["ballValid"] >> ballValid;
    value["robotValid"] >> robotValid;
    value["ballIsFree"] >> ballIsFree;
  }
};
