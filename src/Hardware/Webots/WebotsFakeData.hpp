#pragma once

#include "Hardware/FakeDataInterface.hpp"
#include <mutex>
#include <vector>

class WebotsInterface;
// NOLINTNEXTLINE(readability-identifier-naming)
namespace webots
{
  class Node;
} // namespace webots

class WebotsFakeData : public FakeDataInterface
{
public:
  explicit WebotsFakeData(WebotsInterface& robotInterface);
  void lock();
  void unlock();
  bool readFakeRobotPose(Pose& fakeData) override;
  bool readFakeBallPosition(Vector2f& fakeData) override;
  bool readFakeRobotPositions(VecVector2f& fakeData) override;

private:
  bool getFakeDataInternal(const std::type_index& id, DataTypeBase& data) override;

  WebotsInterface& robotInterface_;
  std::mutex mutex_;
  webots::Node* ball_{nullptr};
  std::vector<webots::Node*> otherRobots_;
};
