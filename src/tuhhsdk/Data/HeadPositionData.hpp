#pragma once

#include "Framework/DataType.hpp"

struct HeadPosition : public Uni::To, public Uni::From
{
  float yaw = 0.f;
  float pitch = 0.f;
  float score = 0.f;

  HeadPosition(float y = 0.f, float p = 0.f, float s = 0.f)
    : yaw(y)
    , pitch(p)
    , score(s)
  {
  }

  virtual void toValue(Uni::Value& value) const
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["yaw"] << yaw;
    value["pitch"] << pitch;
    value["score"] << score;
  }

  virtual void fromValue(const Uni::Value& value)
  {
    value["yaw"] >> yaw;
    value["pitch"] >> pitch;
    value["score"] >> score;
  }
};

class HeadPositionData : public DataType<HeadPositionData>
{
public:
  /// the name of this DataType
  DataTypeName name = "HeadPositionData";
  /// a head position to track or find the ball
  HeadPosition ballAndLocalizationHeadPosition;
  /// a head position to use the head for localization purposes
  HeadPosition localizationHeadPosition;
  /// a head position to look around
  HeadPosition lookAroundHeadPosition;
  /// a head position to look around the ball
  HeadPosition lookAroundBallHeadPosition;
  /// a head position to track the ball
  HeadPosition trackBallHeadPosition;
  /// a head position which is used as middle point for the lookaround behavior
  HeadPosition HeadPositionToExplore;
  /**
   * @brief reset values to invalid value.
   */
  void reset()
  {
    ballAndLocalizationHeadPosition = HeadPosition();
    localizationHeadPosition = HeadPosition();
    lookAroundHeadPosition = HeadPosition();
    trackBallHeadPosition = HeadPosition();
  }

  virtual void toValue(Uni::Value& value) const
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["ballHeadPosition"] << ballAndLocalizationHeadPosition;
    value["localizationHeadPosition"] << localizationHeadPosition;
    value["lookAroundHeadPosition"] << lookAroundHeadPosition;
    value["trackBallHeadPosition"] << trackBallHeadPosition;
  }

  virtual void fromValue(const Uni::Value& value)
  {
    value["ballHeadPosition"] >> ballAndLocalizationHeadPosition;
    value["localizationHeadPosition"] >> localizationHeadPosition;
    value["lookAroundHeadPosition"] >> lookAroundHeadPosition;
    value["trackBallHeadPosition"] >> trackBallHeadPosition;
  }
};
