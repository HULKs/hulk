#pragma once

#include "Data/BallState.hpp"
#include "Modules/NaoProvider.h"
#include "Tools/Math/Angle.hpp"
#include "Tools/Math/Eigen.hpp"
#include "Tools/Math/Pose.hpp"


namespace BallUtils
{
  /**
   * @brief kickPose calculates the desired Pose to kick the ball to a target
   * @param ballSource the current relative ball position
   * @param ballTarget the relative position where the ball should end up
   * @param distanceToBall the desired distance between the robot origin and the ball in x direction [m]
   * @param lastSign the sign of the foot that should kick in the last decision (1 for left, -1 for right)
   * @param forceSign whether lastSign must not be changed (can be used to enforce kicking with a specific foot)
   * @return the resulting relative kick pose
   */
  inline Pose kickPose(const Vector2f& ballSource, const Vector2f& ballTarget, const float distanceToBall, int& lastSign, const bool forceSign = false)
  {
    const Vector2f sourceToTarget = ballTarget - ballSource;

    // We want to stand behind the ball
    const Vector2f behindBall = sourceToTarget.normalized() * (-distanceToBall);
    // We want to choose the correct foot
    const float sourceTargetDistance = (ballTarget.x() * ballSource.y() - ballTarget.y() * ballSource.x()) / sourceToTarget.norm();
    if (!forceSign && std::abs(sourceTargetDistance) > 0.05f)
    {
      lastSign = sourceTargetDistance > 0.f ? 1 : -1;
    }
    const Vector2f footSelect1 = (ballTarget - ballSource).normalized() * 0.05f;
    const Vector2f footSelect(lastSign * footSelect1.y(), -lastSign * footSelect1.x());


    return Pose(ballSource + behindBall + footSelect, std::atan2(sourceToTarget.y(), sourceToTarget.x()));
  }


  enum Kickable
  {
    /// ball is kickable with right foot
    RIGHT,
    /// ball is kickable with left foot
    LEFT,
    /// ball is not kickable at the moment
    NOT
  };

  /**
   * @brief kickable determines whether and with which foot a ball is kickable
   * @param kickPose the relative kick pose
   * @param ballState the state of the own ball
   * @param distanceToBall the desired distance between the robot origin and the ball in x direction [m]
   * @param angleToBall the angle threshold for the orientation check [rad]
   * @return the way the ball is currently kickable
   */
  inline Kickable kickable(const Pose& kickPose, const BallState& ballState, const float distanceToBall, const float angleToBall)
  {
    const float kickableBallAgeMax = 0.3f;
    if (ballState.found && ballState.age < kickableBallAgeMax)
    {
      const bool nearRight = std::abs(ballState.position.y() + 0.05f) < 0.01f && std::abs(ballState.position.x() - distanceToBall) < 0.01f;
      const bool nearLeft = std::abs(ballState.position.y() - 0.05f) < 0.01f && std::abs(ballState.position.x() - distanceToBall) < 0.01f;
      // zero because x axis is facing forward (relative to robot) => angle 0
      bool correctDirection = Angle::angleDiff(0, kickPose.orientation) < angleToBall;

      if (nearLeft && correctDirection)
      {
        return Kickable::LEFT;
      }
      else if (nearRight && correctDirection)
      {
        return Kickable::RIGHT;
      }
    }
    return Kickable::NOT;
  }
}
