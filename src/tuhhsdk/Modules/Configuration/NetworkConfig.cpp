#include "NetworkConfig.hpp"
#include "ConfigMessageHeader.h"

// ASIO defines an macro  #define ERROR 0 which failes with our enum LogLevel::ERROR
#include <boost/asio.hpp>
#include "Definitions/windows_definition_fix.hpp"

#include <boost/bind.hpp>
#include <boost/array.hpp>

#include <Libs/json/json.h>

#include <thread>
#include <vector>
#include <memory>

#include <Modules/Configuration/Configuration.h>
#include <Tools/Storage/UniValue/UniValue2Json.hpp>

#include "print.h"

class NetworkConfig::Impl :
    public std::enable_shared_from_this<NetworkConfig::Impl>
{
private:
  boost::asio::io_service ioService_;
  boost::asio::ip::tcp::endpoint serverEndpoint_;
  boost::asio::ip::tcp::acceptor acceptor_;
  boost::asio::ip::tcp::socket   socket_;

  std::shared_ptr<std::thread> backgroundThread_;

  Configuration& config_;
  bool isConnected_;
  std::vector< char > headerBuffer_;
  std::vector< char > bodyBuffer_;
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
  Impl(const uint16_t& port, Configuration& config);
  ~Impl();

  void startBackgroundThread();
};

//================================
// pimpl
//================================
NetworkConfig::Impl::Impl(const uint16_t& port, Configuration& config)
  : ioService_()
  , serverEndpoint_(boost::asio::ip::tcp::v4(), port)
  , acceptor_(ioService_, serverEndpoint_)
  , socket_(ioService_)
  , config_(config)
  , isConnected_(false)
  , headerBuffer_(sizeof(ConfigMessageHeader))
  , bodyBuffer_()
{
  startAccept();
}

NetworkConfig::Impl::~Impl()
{
  ioService_.stop();
  backgroundThread_->join();
  socket_.close();
}

void NetworkConfig::Impl::startBackgroundThread()
{
  backgroundThread_ = std::make_shared<std::thread>([this](){
    ioService_.run();
    print( "Shutting down transceiver thread", LogLevel::DEBUG );
  });
}

void NetworkConfig::Impl::startAccept()
{
  if ( isConnected_ ) return;
  print( "NetworkConfig: Waiting for connection", LogLevel::INFO );
  socket_ = boost::asio::ip::tcp::socket(ioService_);
  acceptor_.async_accept(socket_,
                         boost::bind(&Impl::onConnect, this,
                                     boost::asio::placeholders::error));
}

void NetworkConfig::Impl::onConnect(const boost::system::error_code& /*error*/)
{
  print( "NetworkConfig: connected", LogLevel::INFO );
  isConnected_ = true;
  receive();
}

void NetworkConfig::Impl::onDisonnect()
{
  print( "NetworkConfig: disconnect", LogLevel::INFO );
  isConnected_ = false;
  startAccept();
}


void NetworkConfig::Impl::receive()
{
  print( "NetworkConfig: receiving header", LogLevel::INFO );
  boost::asio::async_read(socket_,
                          boost::asio::buffer(headerBuffer_),
                          boost::bind(
                            &Impl::onReceiveHeader, this,
                            boost::asio::placeholders::error
                            )
                          );
}

void NetworkConfig::Impl::onReceiveHeader(const boost::system::error_code& error)
{
  if ( (boost::asio::error::eof == error) ||
       (boost::asio::error::connection_reset == error) )
  {
    onDisonnect();
    return;
  }
  if ( error )
  {
    print( "NetworkConfig: error while receiving header", LogLevel::WARNING );
    return;
  }

  print( "NetworkConfig: received header", LogLevel::INFO );
  std::memcpy( &header_, headerBuffer_.data(), sizeof header_ );

  bodyBuffer_.resize(header_.msgLength);

  boost::asio::async_read(socket_,
                          boost::asio::buffer(bodyBuffer_),
                          boost::bind(
                            &Impl::onReceiveBody, this,
                            boost::asio::placeholders::error
                            )
                          );
}

void NetworkConfig::Impl::onReceiveBody(const boost::system::error_code& error)
{
  if ( error )
  {
    print( "NetworkConfig: error while receiving body", LogLevel::WARNING );
    return;
  }
  std::string body(bodyBuffer_.begin(), bodyBuffer_.end());

  if ( header_.msgType == CM_SET )
  {
    print( "NetworkConfig: received message type CM_SET: "+body, LogLevel::INFO );

    Json::Reader reader;
    Json::Value root;

    if ( ! reader.parse(body, root) && root.isArray() )
    {
      print( "NetworkConfig: body is not valid json", LogLevel::WARNING );
      receive();
      return;
    }

    for( auto it = root.begin(); it != root.end(); ++it ) {
      Json::Value item = *it;
      if ( ! item.isObject() ) {
        print( "NetworkConfig: set body contains malformed array element", LogLevel::WARNING);
        continue;
      }
      try {
        config_.set(item.get("mp", "").asString(), item.get("key", "").asString(),
          Uni::Converter::toUniValue(item.get("value", "")));
      } catch (const ConfigurationException& e) {
        print(std::string("NetworkConfig: Exception from Configuration: ") + e.what(), LogLevel::ERROR);
      }
    }
  }
  if ( header_.msgType == CM_GET_MOUNTS )
  {
    print( "NetworkConfig: received message type CM_GET_MOUNTS: "+body, LogLevel::INFO );
    transmitMountList();
  }
  if ( header_.msgType == CM_GET_KEYS )
  {
    print( "NetworkConfig: received message type CM_GET_KEYS: " + body, LogLevel::INFO);
    transmitKeyList(body);
  }
  if ( header_.msgType == CM_SAVE )
  {
    print( "NetworkConfig: received message type CM_SAVE: "+body, LogLevel::INFO );
    try {
      config_.save();
    } catch (const ConfigurationException& e) {
      print(std::string("NetworkConfig: Exception from Configuration: ") + e.what(), LogLevel::ERROR);
    }
  }

  receive();
}

void NetworkConfig::Impl::transmitMountList(){
  Json::Value root;
  Json::Value jsonArray(Json::arrayValue);
  auto keys = config_.getMountPoints();

  for(auto it = keys.begin(); it != keys.end(); ++it){
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
      [this, self, json](boost::system::error_code error, std::size_t /*length*/)
    {
      if ( error )
      {
        Log(LogLevel::WARNING) << "TCPTransport: error while sending List, disconnecting...";
        return;
      }

      Log(LogLevel::DEBUG) <<  "TCPTransport: sent List.";
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

void NetworkConfig::Impl::transmitKeyList(std::string mountPoint){
  Json::Value root;
  Json::Value jsonArray(Json::arrayValue);
  auto keys = config_.get(mountPoint);

  for(auto it = keys.objectBegin(); it != keys.objectEnd(); ++it){
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
      [this, self, json](boost::system::error_code error, std::size_t /*length*/)
    {
      if ( error )
      {
        Log(LogLevel::WARNING) << "TCPTransport: error while sending List, disconnecting...";
        return;
      }

      Log(LogLevel::DEBUG) <<  "TCPTransport: sent List.";
    });
  }
  catch (boost::system::system_error&)
  {
    print("Error transmitting debug updates!", LogLevel::ERROR);
  }
}

//================================
// NetworkConfig
//================================
NetworkConfig::NetworkConfig(const std::uint16_t& port, Configuration& config)
{
  pimpl_.reset(new Impl(port, config));
}

NetworkConfig::~NetworkConfig()
{
}

void NetworkConfig::run()
{
  pimpl_->startBackgroundThread();
}
