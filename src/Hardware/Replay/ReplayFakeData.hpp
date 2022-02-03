#pragma once

#include <array>
#include <condition_variable>
#include <mutex>

#include "Data/HeadMatrixBuffer.hpp"
#include "Data/ReplayData.hpp"
#include "Hardware/FakeDataInterface.hpp"

class ReplayFakeData final : public FakeDataInterface
{
public:
  /**
   * @brief readFakeRobotPose getter for the faked absolute pose of the robot provided by the
   * fakeDataInterface
   * @param fakeData a reference to the value to store the fake data to
   * @return true if fake data can be provided
   */
  bool readFakeRobotPose(Pose& fakeData) override;
  /**
   * @brief readFakeBallPosition getter for the fake relative position of the ball provided by the
   * fakeDataInterface
   * @param fakeData a reference to the value to store the fake data to
   * @return true if fake data can be provided
   */
  bool readFakeBallPosition(Vector2f& fakeData) override;
  /**
   * @brief readFakeRobotPositions getter for the fake relative positions of other robots provided
   * by the fakeDataInterface
   * @param fakeData a reference to the value to store the fake datat to
   * @return true if fake data can be provided
   */
  bool readFakeRobotPositions(VecVector2f& fakeData) override;

  /// the configurations recorded from the ReplayRecorder
  ReplayConfigurations replayConfig;
  /// the current frame which is played
  ReplayFrame currentFrame;

private:
  bool getFakeDataInternal(const std::type_index& id, DataTypeBase& data) override;
};
