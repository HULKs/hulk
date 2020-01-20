#pragma once

#include <mutex>
#include <vector>
#include <queue>
#include <string>
#include <array>
#include <thread>
#include <condition_variable>

#include <boost/bind.hpp>
#include <boost/shared_ptr.hpp>
#include <boost/enable_shared_from_this.hpp>
#include <boost/asio.hpp>
#include "Definitions/windows_definition_fix.hpp"

#include <QObject>
#include <QtWebSockets/QWebSocketServer>
#include <QtCore/QList>
#include <QtCore/QByteArray>
//#include "QtWebsocketInterface.moc"

QT_FORWARD_DECLARE_CLASS(QWebSocketServer)
QT_FORWARD_DECLARE_CLASS(QWebSocket)

#include "Tools/Math/Vector3.h"

#include "Hardware/RobotInterface.hpp"
#include "QtWebsocketCamera.hpp"
#include "print.h"

//////////////////
/// \brief The TcpServer class
///

class QtWebsocketInterface;

class TcpServer : public QObject {
    Q_OBJECT

public:
  TcpServer(QtWebsocketInterface* robotInterface, QObject * parent = 0);
  ~TcpServer();

public slots:
  void acceptConnection();
  void processTextMessage(QString message);
  void processBinaryMessage(QByteArray message);
  void socketDisconnected();
private:
  QWebSocketServer* server;
  QList<QWebSocket*> clients;
  QtWebsocketInterface* robotInterface;
};

//////////////////




class QtWebsocketInterface : public RobotInterface {
public:
  /**
   * @brief QWebsocketsInterface constructs a thread with a websocket server
   */
  QtWebsocketInterface(int argc, char** argv);
  /**
   * @brief configure does nothing
   */
  void configure(Configuration&, NaoInfo&) override;
  /**
   * @brief setJointAngles does nothing
   */
  void setJointAngles(const std::vector<float>& angles) override;
  /**
   * @brief setJointStiffnesses does nothing
   */
  void setJointStiffnesses(const std::vector<float>& stiffnesses) override;
  /**
   * @brief setLEDs does nothing
   */
  void setLEDs(const std::vector<float>& leds) override;
  /**
   * @brief setSonar does nothing
   */
  void setSonar(const float sonar) override;
  /**
   * @brief waitAndReadSensorData transmits joint commands, simulates a cycle and gets new sensor data
   * @param data is filled with sensor data from the websocket
   * @return duration between the last and current received sensor data
   */
  float waitAndReadSensorData(NaoSensorData& data) override;
  /**
   * @brief getFileRoot returns a path to a directory that contains all files for our program
   * @return a path
   */
  std::string getFileRoot() override;
  /**
   * @brief getNaoInfo copies the hardware identification
   * @param info is filled with the body/head version and name
   */
  void getNaoInfo(Configuration&, NaoInfo& info) override;

  CameraInterface& getCamera(const Camera camera) override;

  // todo: Missing documentation x3
  CameraInterface& getCurrentCamera();

  void updateAccelData(float x, float y, float z);
  void updateGyroData(float x, float y, float z);

private:
    QtWebsocketCamera topCamera;
    QtWebsocketCamera bottomCamera;
    /// mutex to lock DCM commands of different threads
    std::mutex mutex_;
    /// accelerometer of simulated robot
    std::queue<Vector3<float>> accelerometer_;
    /// gyroscope of simulated robot
    std::queue<Vector3<float>> gyroscope_;

    std::thread qtMainThread;

    std::mutex updateLock;

    std::condition_variable sensorUpdateAvailable;
    std::mutex sensorUpdateMutex;
};
