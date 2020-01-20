#pragma once

#include <memory>

#include <boost/interprocess/mapped_region.hpp>
#include <boost/interprocess/shared_memory_object.hpp>

#include "Hardware/Nao/common/NaoAudio.hpp"
#include "Hardware/Nao/common/SMO.h"
#include "Hardware/Nao/v5/Nao5Camera.hpp"
#include "Hardware/RobotInterface.hpp"

#include "Hardware/Nao/common/NaoFakeData.hpp"

class NaoInterface : public RobotInterface
{
public:
  /**
   * @brief NaoInterface connects to the shared memory of the ALModule
   */
  NaoInterface();
  /**
   * @brief ~NaoInterface
   */
  ~NaoInterface();

  void configure(Configuration&, NaoInfo&) override;
  void setJointAngles(const std::vector<float>& angles) override;
  void setJointStiffnesses(const std::vector<float>& stiffnesses) override;
  void setLEDs(const std::vector<float>& leds) override;
  void setSonar(const float sonar) override;
  float waitAndReadSensorData(NaoSensorData& data) override;
  std::string getFileRoot() override;
  std::string getDataRoot() override;
  void getNaoInfo(Configuration&, NaoInfo& info) override;
  CameraInterface& getCamera(const Camera camera) override;
  AudioInterface& getAudio() override;
  CameraInterface& getNextCamera() override;
  Camera getCurrentCameraType() override;
  FakeDataInterface& getFakeData() override;

private:
  /**
   * @brief initNaoInfo converts IDs and version strings to names and enums
   * @param config a reference to the Configuration instance
   */
  void initNaoInfo(Configuration& config);
  /// Shared memory
  boost::interprocess::shared_memory_object segment_;
  // todo: Missing documentation x11
  boost::interprocess::mapped_region region_;
  SharedBlock* shmBlock_;
  std::array<char[64], keys::naoinfos::NAOINFO_MAX> rawInfo_;
  NaoInfo naoInfo_;
  Nao5Camera topCamera_;
  Nao5Camera bottomCamera_;
  NaoAudio audioInterface_;
  NaoFakeData fakeData_;
  Camera currentCamera_;
  uint64_t currentUsedImageTimeStamp;
  uint64_t lastUsedImageTimeStamp;
};
