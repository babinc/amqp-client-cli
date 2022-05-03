# amqp-client-cli
***
![](https://s3.us-west-2.amazonaws.com/www.carmanbabin.com/amqp-client-cli/amqp-client-cli.gif)

Why CLI and not GUI:
 - Message bus servers often do not have a desktop environment installed, therefore in order to be a good
debugging tool it needs to run without a GUI.
 - Terminal is cooler than GUI 😜
 
Why Rust:
 - Cross-platform support
 - Great performance
 - Good error handling
 - I enjoy it more than other languages 😁

## Cargo Installer
If you have Cargo/Rust installed you can install the program with cargo using the command below:

```cargo install amqp-client-cli```

## Windows Installer
[amqp-client-cli-0.1.5-setup.exe](https://s3.us-west-2.amazonaws.com/www.carmanbabin.com/amqp-client-cli/amqp-client-cli-0.1.5-setup.exe)

## Linux Installer

```sudo snap install amqp-client-cli```

## Config File
The program must have a valid configuration file in order to run.

Upon starting ```amqp-client-cli``` will look in the following locations for a configuration file.
1. (Optional) Argument Path
    - Example: ```amqp-client-cli ~/projects/test.json```
2. (Automatic) Local Path
    - The current directory of execution.
    - Example: ```amqp-client-cli ./amqp-client-cli.json```
3. (Automatic) Config Path
   - Linux:   ```/home/Carman/.config/amqp-client-cli.json```
   - Windows: ```C:\Users\Carman\AppData\Roaming\amqp-client-cli.json```
   - macOS:   ```/Users/Carman/Library/Application Support/amqp-client-cli.json```
4. (Automatic) Home Path
    - Linux:   ```/home/Carman/amqp-client-cli.json```
    - Windows: ```C:\Users\Carman\amqp-client-cli.json```
    - macOS:   ```/Users/Carman/amqp-client-cli.json```

## Config File Example
```json
{
  "host": "127.0.0.1",
  "port": 5672,
  "username": "guest",
  "password": "guest",
  "pfx_path": null,
  "pem_file": null,
  "domain": "test-domain",
  "items": [
    {
      "exchange_name": "test_program.incoming",
      "exchange_type": "Topic",
      "queue_routing_key": "*.*.*.*.#",
      "alias": "Incoming",
      "pretty": true,
      "log_file": "/tmp/logs.txt"
    },
    {
      "exchange_name": "test_program.logs",
      "exchange_type": "Topic",
      "queue_routing_key": "*.*.*.*.#",
      "alias": "Logs",
      "pretty": false,
      "log_file": "/tmp/logs.txt"
    },
    {
      "exchange_name": "test_program.trade",
      "exchange_type": "Topic",
      "queue_routing_key": "*.*.*.*.#",
      "alias": "Trade",
      "pretty": false,
      "log_file": null
    }
  ]
}

```
## Edit
The user can set edit options for an exchange by pressing the ```E``` key. Each exchange has its own set of options. Or the
user can set the options in the config file. When the program exits the options that were set while using the
program will be written to the configuration file.
## Scrolling
While using the program the user can press the ```P``` key to pause the program. Once the program is paused no more
messages will automatically appear in the Messages Window. Then the user can press the Up and Down arrow keys or the Page
Up and Page Down keys to scroll the text in the messages window.
## Logging
The user can either set the logging parameter in the Configuration File or do it from within the program using the Options
Window. Once a log file path is set the program will write out the messages for the exchange that it was set for to the file. The user can also
add multiple exchanges to a single log file in order to log more than one exchange to a single file. Or you can log them
into separate files. Logs are written to the file once every second. 
## Queue's
amqp-client-cli leaves all existing queue's in place on the server. When subscribing to an exchange a new queue will be
created, and when unsubscribing the newly created queue will be deleted. 
## TODO
  - Clean up and breakdown the UI Struct
  - Let users decide to color messages from certain exchanges
  - Unit Testing
  - Add ability to publish
  - Add more protocols
      - MQTT
