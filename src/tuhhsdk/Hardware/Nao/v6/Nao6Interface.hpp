#pragma once

#include <condition_variable>
#include <memory>
#include <mutex>
#include <thread>

#include <boost/array.hpp>
#include <boost/asio.hpp>
#include <boost/system/error_code.hpp>

#include <msgpack.hpp>

#include "Hardware/Nao/common/BatteryDisplay.hpp"
#include "Hardware/Nao/common/NaoAudio.hpp"
#include "Hardware/Nao/common/NaoFakeData.hpp"
#include "Hardware/Nao/common/SMO.h"
#include "Hardware/Nao/v6/Nao6Camera.hpp"
#include "Hardware/RobotInterface.hpp"

#include "Tools/Var/SpscQueue.hpp"

constexpr const size_t LoLADatumSize = 896;

typedef boost::array<char, 8000> LoLADataBuffer;
typedef boost::array<char, LoLADatumSize> LoLASingleDatumBuffer;

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
  ~NaoInterface() override;

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
   * @brief registerForSocketReceive
   */
  void registerForSocketReceive();
  /**
   * @brief initNaoInfo converts IDs and version strings to names and enums
   * @param config a reference to the Configuration instance
   */
  void initNaoInfo(Configuration& config);

  /**
   * @brief extractJoints extracts the joint data from a given messagePack object.
   * @param array the messagePack object to parse the joint data from
   * @param jointData the array to store the parsed data in.
   */
  void extractJoints(msgpack::object& array,
                     std::array<float, keys::joints::JOINTS_MAX>& jointData);
  /**
   * @brief extractVector2 extracts a two dimensional floating point vector from the given
   * messagePack object.
   * @param array the messagePack object to parse the vector from
   * @param dest the float array to put the data in.
   */
  void extractVector2(msgpack::object& array, float* dest);
  /**
   * @brief extractVector3 extracts a three dimensional floating point vector from the given
   * messagePack object.
   * @param array the messagePack object to parse the vector from
   * @param dest the float array to put the data in
   */
  void extractVector3(msgpack::object& array, float* dest);
  /**
   * @brief extractFSRs extracts the Force Sensing Resistor data from the given messagePack object.
   * @param array the messagePack object to parse the vector from
   * @param leftFSR the array to put the data for the left foot in
   * @param rightFSR the array to put the data for the right foot in
   */
  void extractFSRs(msgpack::object& array, std::array<float, keys::sensor::FSR_MAX>& leftFSR,
                   std::array<float, keys::sensor::FSR_MAX>& rightFSR);
  /**
   * @brief extractBattery extracts the battery info (charge, current, ...) from the given
   * messagePack object.
   * @param array the messagePack object to parse the battery data from
   * @param battery the array to put the battery data in
   */
  void extractBattery(msgpack::object& array,
                      std::array<float, keys::sensor::BATTERY_MAX>& battery);
  /**
   * @brief extractSwitches extracts the button data from the given messagePack object.
   * @param array the messagePack object to parse the button data from
   * @param switches the array to put the button data in
   */
  void extractSwitches(msgpack::object& array,
                       std::array<float, keys::sensor::SWITCH_MAX>& switches);
  /**
   * @brief extractSonar extracts the sonar sensor data from the given messagePack object.
   * @param array the messagePack object to parse the sonar sensor data from
   * @param sonar the array to put the sonar sensor data in
   */
  void extractSonar(msgpack::object& array, std::array<float, keys::sensor::SONAR_MAX>& sonar);

  void packJoints(msgpack::packer<msgpack::sbuffer>& packer,
                  std::array<float, keys::joints::JOINTS_MAX>& jointData, std::string name);
  void packFloatArray(msgpack::packer<msgpack::sbuffer>& packer, float* src,
                      std::vector<int>& remapping, std::string name);

  std::vector<keys::joints::enumJoints> jointsRemapping_;
  std::vector<keys::sensor::battery> batteryRemapping_;
  std::vector<keys::sensor::switches> switchesRemapping_;
  std::vector<int> colorRemapping_;
  std::vector<int> lEarRemapping_;
  std::vector<int> rEarRemapping_;
  std::vector<int> skullRemapping_;
  std::vector<int> lEyeRemapping_;
  std::vector<int> rEyeRemapping_;

  BatteryDisplay batteryDisplay_;
  SharedBlock dataBlock_;
  /// Whether the background thread is out of sync with LoLA
  bool lolaDesync_;
  /// The size of the fragment that was received
  std::size_t fragmentSize_;
  /// Data of the fragment that was received
  LoLASingleDatumBuffer fragment_;
  /// The last datum that was successfully popped from the ringbuffer
  LoLASingleDatumBuffer lastLolaReceivedDatum_;

  /// boost::array which stores received data
  std::shared_ptr<LoLADataBuffer> receive_;
  /// boost::array which stores sent data
  std::shared_ptr<LoLADataBuffer> send_;
  /// boost::asio IO service that runs in its seperate thread
  boost::asio::io_service ioService_;
  /// UDP socket
  boost::asio::local::stream_protocol::socket socket_;
  /// UDP endpoint for LoLA
  boost::asio::local::stream_protocol::endpoint lolaEndpoint_;
  /// the thread in which the asio IO service runs
  std::shared_ptr<std::thread> backgroundThread_;
  /// mutex to prevent race conditions between the cycle and the asynchronous parts of this class
  std::mutex mutex_;
  std::condition_variable newNetworkDataCondition_;
  /// whether new network data came in
  bool newNetworkData_;
  TimePoint timeNetworkDataReceived_;
  /// simple msgpack buffer for data that will be sent to LoLA
  msgpack::sbuffer sbuf_;

  /// Chest button state in the last cycle
  float previousChestButtonState_;
  float previousFrontHeadState_;
  float previousRearHeadState_;
  /// Time when the chest button was last pressed. Used to wait for double press.
  std::chrono::time_point<std::chrono::system_clock> previousFrontHeadTime_;
  std::chrono::time_point<std::chrono::system_clock> previousRearHeadTime_;
  /// Whether to publish the recognized chest button action. Used to wait for double press.
  bool sentChestButton_;

  NaoInfo naoInfo_;
  NaoFakeData fakeData_;
  NaoAudio audioInterface_;
  Nao6Camera topCamera_;
  Nao6Camera bottomCamera_;
  Camera currentCamera_;

  /**
   * @brief single input single consumer ring buffer for LoLA data.
   * Used by the background thread for storing received packages
   * Used by motion to get new data whenever it is available
   */
  SpscRing<LoLASingleDatumBuffer, 100> lolaData_;

  uint64_t currentUsedImageTimeStamp;
  uint64_t lastUsedImageTimeStamp;
};
