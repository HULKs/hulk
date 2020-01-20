#pragma once

#include <array>
#include <condition_variable>
#include <mutex>

#include "Hardware/FakeDataInterface.hpp"

class NaoFakeData final : public FakeDataInterface
{
public:
  /**
   * @brief the constructor of this class
   */
  NaoFakeData(){};
  /**
   * @brief the destructor of this class
   */
  virtual ~NaoFakeData(){};
  /**
   * @brief waitForFakeData waits until there is a new set of fake data is available to be processed
   */
  virtual void waitForFakeData();
  /**
   * @brief readFakeRobotPose getter for the faked absolute pose of the robot provided by the
   * fakeDataInterface
   * @param fakeData a reference to the value to store the fake data to
   * @return true if fake data can be provided
   */
  virtual bool readFakeRobotPose(Pose& fakeData);
  /**
   * @brief readFakeBallPosition getter for the fake relative position of the ball provided by the
   * fakeDataInterface
   * @param fakeData a reference to the value to store the fake data to
   * @return true if fake data can be provided
   */
  virtual bool readFakeBallPosition(Vector2f& fakeData);
  /**
   * @brief readFakeRobotPositions getter for the fake relative positions of other robots provided
   * by the fakeDataInterface
   * @param fakeData a reference to the value to store the fake datat to
   * @return true if fake data can be provided
   */
  virtual bool readFakeRobotPositions(VecVector2f& fakeData);

private:
  virtual bool getFakeDataInternal(const std::type_index&, DataTypeBase&) {
    return false;
  }
};
