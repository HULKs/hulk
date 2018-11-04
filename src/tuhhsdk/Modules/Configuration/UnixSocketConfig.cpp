#include "UnixSocketConfig.hpp"
#include "ConfigMessageHeader.h"

// ASIO defines an macro  #define ERROR 0 which fails with our enum LogLevel::ERROR
#include "Definitions/windows_definition_fix.hpp"
#include <boost/asio.hpp>
#include <boost/filesystem.hpp>

#include <boost/array.hpp>
#include <boost/bind.hpp>

#include <Libs/json/json.h>

#include <memory>
#include <thread>
#include <vector>

#include <Modules/Configuration/Configuration.h>
#include <Tools/Storage/UniValue/UniValue2Json.hpp>

#include "print.h"

#ifndef _WIN32


class UnixSocketConfig::Impl : public std::enable_shared_from_this<UnixSocketConfig::Impl>
{
private:
  boost::asio::io_service ioService_;
  boost::asio::local::stream_protocol::endpoint serverEndpoint_;
  boost::asio::local::stream_protocol::acceptor acceptor_;
  boost::asio::local::stream_protocol::socket socket_;

  std::shared_ptr<std::thread> backgroundThread_;

  Configuration& config_;
  bool isConnected_;
  std::vector<char> headerBuffer_;
  std::vector<char> bodyBuffer_;
  ConfigMessageHeader header_;

  void startAccept();
  void onConnect(const boost::system::error_code& error);
  void onDisonnect();

  void receive();
  void onReceiveHeader(const boost::system::error_code& /*error*/);
  void onReceiveBody(const boost::system::error_code& /*error*/);
  void transmitMountList();
  void transmitKeyList(std::string mountPoint);

public:
  Impl(const std::string& file, Configuration& config);
  ~Impl();

  void startBackgroundThread();
};

//================================
// pimpl
//================================
UnixSocketConfig::Impl::Impl(const std::string& file, Configuration& config)
  : ioService_()
  , serverEndpoint_(file)
  , acceptor_(ioService_, serverEndpoint_)
  , socket_(ioService_)
  , config_(config)
  , isConnected_(false)
  , headerBuffer_(sizeof(ConfigMessageHeader))
  , bodyBuffer_()
{
  startAccept();
}

UnixSocketConfig::Impl::~Impl()
{
  ioService_.stop();
  backgroundThread_->join();
  socket_.close();
}

void UnixSocketConfig::Impl::startBackgroundThread()
{
  backgroundThread_ = std::make_shared<std::thread>([this]() {
    ioService_.run();
    print("Shutting down transceiver thread", LogLevel::INFO);
  });
}

void UnixSocketConfig::Impl::startAccept()
{
  if (isConnected_)
    return;
  print("UnixSocketConfig: Waiting for connection", LogLevel::DEBUG);
  socket_ = boost::asio::local::stream_protocol::socket(ioService_);
  acceptor_.async_accept(socket_,
                         boost::bind(&Impl::onConnect, this, boost::asio::placeholders::error));
}

void UnixSocketConfig::Impl::onConnect(const boost::system::error_code& /*error*/)
{
  print("UnixSocketConfig: connected", LogLevel::INFO);
  isConnected_ = true;
  receive();
}

void UnixSocketConfig::Impl::onDisonnect()
{
  print("UnixSocketConfig: disconnect", LogLevel::INFO);
  isConnected_ = false;
  startAccept();
}


void UnixSocketConfig::Impl::receive()
{
  print("UnixSocketConfig: receiving header", LogLevel::DEBUG);
  boost::asio::async_read(
      socket_, boost::asio::buffer(headerBuffer_),
      boost::bind(&Impl::onReceiveHeader, this, boost::asio::placeholders::error));
}

void UnixSocketConfig::Impl::onReceiveHeader(const boost::system::error_code& error)
{
  if ((boost::asio::error::eof == error) || (boost::asio::error::connection_reset == error))
  {
    onDisonnect();
    return;
  }
  if (error)
  {
    print("UnixSocketConfig: error while receiving header", LogLevel::ERROR);
    return;
  }

  print("UnixSocketConfig: received header", LogLevel::DEBUG);
  std::memcpy(&header_, headerBuffer_.data(), sizeof header_);

  bodyBuffer_.resize(header_.msgLength);

  boost::asio::async_read(
      socket_, boost::asio::buffer(bodyBuffer_),
      boost::bind(&Impl::onReceiveBody, this, boost::asio::placeholders::error));
}

void UnixSocketConfig::Impl::onReceiveBody(const boost::system::error_code& error)
{
  if (error)
  {
    print("UnixSocketConfig: error while receiving body", LogLevel::ERROR);
    return;
  }
  std::string body(bodyBuffer_.begin(), bodyBuffer_.end());

  if (header_.msgType == CM_SET)
  {
    print("UnixSocketConfig: received message type CM_SET: " + body, LogLevel::DEBUG);

    Json::Reader reader;
    Json::Value root;

    if (!reader.parse(body, root) && root.isArray())
    {
      print("UnixSocketConfig: body is not valid json", LogLevel::WARNING);
      receive();
      return;
    }

    for (auto it = root.begin(); it != root.end(); ++it)
    {
      Json::Value item = *it;
      if (!item.isObject())
      {
        print("UnixSocketConfig: set body contains malformed array element", LogLevel::WARNING);
        continue;
      }
      try
      {
        config_.set(item.get("mp", "").asString(), item.get("key", "").asString(),
                    Uni::Converter::toUniValue(item.get("value", "")));
      }
      catch (const ConfigurationException& e)
      {
        print(std::string("UnixSocketConfig: Exception from Configuration: ") + e.what(),
              LogLevel::ERROR);
      }
    }
  }
  if (header_.msgType == CM_GET_MOUNTS)
  {
    print("UnixSocketConfig: received message type CM_GET_MOUNTS: " + body, LogLevel::DEBUG);
    transmitMountList();
  }
  if (header_.msgType == CM_GET_KEYS)
  {
    print("UnixSocketConfig: received message type CM_GET_KEYS: " + body, LogLevel::DEBUG);
    transmitKeyList(body);
  }
  if (header_.msgType == CM_SAVE)
  {
    print("UnixSocketConfig: received message type CM_SAVE: " + body, LogLevel::DEBUG);
    try
    {
      config_.save();
    }
    catch (const ConfigurationException& e)
    {
      print(std::string("UnixSocketConfig: Exception from Configuration: ") + e.what(),
            LogLevel::ERROR);
    }
  }

  receive();
}

void UnixSocketConfig::Impl::transmitMountList()
{
  Json::Value root;
  Json::Value jsonArray(Json::arrayValue);
  auto keys = config_.getMountPoints();

  for (auto it = keys.begin(); it != keys.end(); ++it)
  {
    Json::Value entry(Json::objectValue);
    entry["key"] = it->first;
    entry["filename"] = it->second;
    jsonArray.append(entry);
  }
  root["keys"] = jsonArray;

  Json::FastWriter jsonWriter;
  std::shared_ptr<std::string> json = std::make_shared<std::string>(jsonWriter.write(root));

  ConfigMessageHeader header;
  header.msgType = CM_SEND_MOUNTS;
  header.msgLength = json->length();

  std::vector<boost::asio::const_buffer> sendBuffers;

  sendBuffers.push_back(boost::asio::buffer(&header, sizeof(ConfigMessageHeader)));
  sendBuffers.push_back(boost::asio::buffer(*json));

  try
  {
    // send
    auto self(shared_from_this());
    boost::asio::async_write(socket_, sendBuffers,
                             [self, json](boost::system::error_code error, std::size_t /*length*/) {
                               if (error)
                               {
                                 Log(LogLevel::ERROR)
                                     << "TCPTransport: error while sending List, disconnecting...";
                                 return;
                               }

                               Log(LogLevel::DEBUG) << "TCPTransport: sent List.";
                             });
  }
  catch (boost::system::system_error& e)
  {
    print("Error transmitting debug updates!" + std::string(e.what()), LogLevel::ERROR);
  }
  catch (std::exception& e)
  {
    print("Another error! Oh noes!" + std::string(e.what()), LogLevel::ERROR);
  }
}

void UnixSocketConfig::Impl::transmitKeyList(std::string mountPoint)
{
  Json::Value root;
  Json::Value jsonArray(Json::arrayValue);
  auto keys = config_.get(mountPoint);

  for (auto it = keys.objectBegin(); it != keys.objectEnd(); ++it)
  {
    Json::Value entry(Json::objectValue);
    entry["key"] = it->first;
    entry["value"] = Uni::Converter::toJson(it->second);
    jsonArray.append(entry);
  }
  root["mountPoint"] = mountPoint;
  root["keys"] = jsonArray;

  Json::FastWriter jsonWriter;
  std::shared_ptr<std::string> json = std::make_shared<std::string>(jsonWriter.write(root));

  ConfigMessageHeader header;
  header.msgType = CM_SEND_KEYS;
  header.msgLength = json->length();

  std::vector<boost::asio::const_buffer> sendBuffers;

  sendBuffers.push_back(boost::asio::buffer(&header, sizeof(ConfigMessageHeader)));
  sendBuffers.push_back(boost::asio::buffer(*json));

  try
  {
    // send
    auto self(shared_from_this());
    boost::asio::async_write(socket_, sendBuffers,
                             [self, json](boost::system::error_code error, std::size_t /*length*/) {
                               if (error)
                               {
                                 Log(LogLevel::ERROR)
                                     << "TCPTransport: error while sending List, disconnecting...";
                                 return;
                               }

                               Log(LogLevel::DEBUG) << "TCPTransport: sent List.";
                             });
  }
  catch (boost::system::system_error&)
  {
    print("Error transmitting debug updates!", LogLevel::ERROR);
  }
}

//================================
// UnixSocketConfig
//================================
UnixSocketConfig::UnixSocketConfig(const std::string& file, Configuration& config)
{
  std::remove(file.c_str());
  boost::filesystem::create_directories(boost::filesystem::path(file).parent_path());
  pimpl_.reset(new Impl(file, config));
}

UnixSocketConfig::~UnixSocketConfig() {}

void UnixSocketConfig::run()
{
  pimpl_->startBackgroundThread();
}

#endif
