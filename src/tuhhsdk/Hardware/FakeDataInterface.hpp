#pragma once

#include <array>
#include <mutex>
#include <typeindex>
#include <vector>

#include "Framework/DataType.hpp"
#include "Tools/Math/Eigen.hpp"
#include "Tools/Math/Pose.hpp"

class FakeDataInterface
{
public:
  /**
   * @brief ~FakeDataInterface a virtual destructor for polymorphism
   */
  virtual ~FakeDataInterface() {}
  /**
   * @brief waitForFakeData waits until there is a new set of fake data is available to be processed
   */
  virtual void waitForFakeData() = 0;
  /**
   * @brief readFakeRobotPose getter for the faked absolute pose of the robot provided by the
   * fakeDataInterface
   * @param fakeData a reference to the value to store the fake data to
   * @return true if fake data can be provided
   */
  virtual bool readFakeRobotPose(Pose& fakeData) = 0;
  /**
   * @brief readFakeBallPosition getter for the fake relative position of the ball provided by the
   * fakeDataInterface
   * @param fakeData a reference to the value to store the fake data to
   * @return true if fake data can be provided
   */
  virtual bool readFakeBallPosition(Vector2f& fakeData) = 0;
  /**
   * @brief readFakeRobotPositions getter for the fake relative positions of other robots provided
   * by the fakeDataInterface
   * @param fakeData a reference to the value to store the fake datat to
   * @return true if fake data can be provided
   */
  virtual bool readFakeRobotPositions(VecVector2f& fakeData) = 0;

  /**
   * @brief generic getter for Datatypes for which is getFakeDataInternal is implemented in the according inteface.
   * @return true if the requested datatype is not is actually provided
   */
  template <typename T>
  bool getFakeData(T& data)
  {
    return getFakeDataInternal(typeid(T), data);
  }

protected:
  /// a mutex to lock the access to the fake data
  std::mutex fakeDataMutex_;

private:
  /**
   * @brief internal generic getter for fakedata, should be implemented by the HardwareFakeDataInterface.
   */
  virtual bool getFakeDataInternal(const std::type_index& id, DataTypeBase& data) = 0;
};
