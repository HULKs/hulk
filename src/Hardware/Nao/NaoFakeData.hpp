#pragma once

#include <array>
#include <condition_variable>
#include <mutex>

#include "Hardware/FakeDataInterface.hpp"

class NaoFakeData final : public FakeDataInterface
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

private:
  bool getFakeDataInternal([[maybe_unused]] const std::type_index& typeId,
                           [[maybe_unused]] DataTypeBase& dataType) override
  {
    return false;
  }
};
