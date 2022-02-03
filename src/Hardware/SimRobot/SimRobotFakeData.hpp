#pragma once

#include <condition_variable>
#include <vector>

#include <SimRobotCore2.h>

#include "Hardware/FakeDataInterface.hpp"
#include "Tools/Math/Eigen.hpp"
#include "Tools/Math/Pose.hpp"


class SimRobotFakeData final : public FakeDataInterface
{
public:
  /**
   * @brief the constructor of this class
   */
  SimRobotFakeData() = default;
  /**
   * @brief the destructor of this class
   */
  ~SimRobotFakeData() override = default;
  /**
   * @brief setFakeRobotPose a setter for the fake robot pose for the simrobot interface
   * @param fakeData the faked robot pose
   */
  void setFakeRobotPose(const Pose& fakeData);
  /**
   * @brief readFakeRobotPose getter for the faked absolute pose of the robot provided by the
   * fakeDataInterface
   * @param fakeData a reference to the value to store the fake data to
   * @return true if fake data can be provided
   */
  bool readFakeRobotPose(Pose& fakeData) override;
  /**
   * @brief setFakeBallPosition setter for the fake relative position of the ball provided by the
   * fakeDataInterface
   * @param fakeData the faked ball position
   */
  void setFakeBallPosition(const Vector2f& fakeData);
  /**
   * @brief readFakeBallPosition getter for the fake relative position of the ball provided by the
   * fakeDataInterface
   * @param fakeData a reference to the value to store the fake data to
   * @return true if fake data can be provided
   */
  bool readFakeBallPosition(Vector2f& fakeData) override;
  /**
   * @brief setFakeRobotPositions setter for the fake relative positions of other robots provided by
   * the fakeDataInterface
   * @param fakeData the faked ball position
   */
  void setFakeRobotPositions(const VecVector2f& fakeData);
  /**
   * @brief readFakeRobotPositions getter for the fake relative positions of other robots provided
   * by the fakeDataInterface
   * @param fakeData a reference to the value to store the fake datat to
   * @return true if fake data can be provided
   */
  bool readFakeRobotPositions(VecVector2f& fakeData) override;

private:
  /// true if a fake robot pose is available
  bool fakeRobotPoseIsAvailable_ = false;
  /// the faked robot pose in terms of (x, y, alpha)
  Pose fakeRobotPose_;
  /// true if a fake ball is available
  bool fakeBallIsAvailable_ = false;
  /// the faked ball position in terms of (x, y)
  Vector2f fakeBallPosition_;
  /// true if fake robot positions (of other robots) are available
  bool fakeRobotPositionsAreAvailable_ = false;
  /// the faked position of other robots
  VecVector2f fakeRobotPositions_;

  bool getFakeDataInternal(const std::type_index& /*id*/, DataTypeBase& /*data*/) override
  {
    return false;
  }
};
