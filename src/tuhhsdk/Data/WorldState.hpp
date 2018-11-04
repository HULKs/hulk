#pragma once

#include "Framework/DataType.hpp"


class WorldState : public DataType<WorldState>
{
public:
  /// the name of this DataType
  DataTypeName name = "WorldState";
  bool ballInOwnHalf = false;
  bool ballInLeftHalf = false;
  bool robotInOwnHalf = false;
  bool robotInLeftHalf = false;
  bool ballValid = false;
  bool robotValid = false;
  bool ballIsFree = false;
  bool ballInCorner = false;
  bool ballInPenaltyArea = false;
  bool ballIsToMyLeft = false;
  bool ballInCenterCircle = false;

  void reset() override
  {
    ballValid = false;
    robotValid = false;
    ballIsFree = false;
  }

  void toValue(Uni::Value& value) const override
  {
    value["ballInOwnHalf"] << ballInOwnHalf;
    value["ballInLeftHalf"] << ballInLeftHalf;
    value["robotInOwnHalf"] << robotInOwnHalf;
    value["robotInLeftHalf"] << robotInLeftHalf;
    value["ballValid"] << ballValid;
    value["robotValid"] << robotValid;
    value["ballIsFree"] << ballIsFree;
    value["ballInCorner"] << ballInCorner;
    value["ballInPenaltyArea"] << ballInPenaltyArea;
    value["ballIsToMyLeft"] << ballIsToMyLeft;
    value["ballInCenterCircle"] << ballInCenterCircle;
  }

  void fromValue(const Uni::Value& value) override
  {
    value["ballInOwnHalf"] >> ballInOwnHalf;
    value["ballInLeftHalf"] >> ballInLeftHalf;
    value["robotInOwnHalf"] >> robotInOwnHalf;
    value["robotInLeftHalf"] >> robotInLeftHalf;
    value["ballValid"] >> ballValid;
    value["robotValid"] >> robotValid;
    value["ballIsFree"] >> ballIsFree;
    value["ballInCorner"] >> ballInCorner;
    value["ballInPenaltyArea"] >> ballInPenaltyArea;
    value["ballIsToMyLeft"] >> ballIsToMyLeft;
    value["ballInCenterCircle"] >> ballInCenterCircle;
  }
};
