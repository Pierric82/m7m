# m7m

## What's this?
It's a simple, Yaml-based flow automation tool. It's playfully called m7m because it's (supposed to be) like n8n, but less. Less complex, less powerful, and much less resource-intensive.
I started working on this as a Rust learning exercise when I had a single important flow running in n8n on my Raspberry Pi, and n8n was causing my Pi to slow down significantly for several seconds every time it ran the flow (every 30s), causing issues in other applications like media playback.
Since then, n8n's performance has dramatically improved and I no longer have issues with it! I've never really put this to use, however the flow I was initially aiming for has been completed with this tool, and was shown to work.

## Which state is it in?
Absolutely pre-alpha: it works with a few features, but unless you want to build exactly the same flow that I wanted, it's going to be missing some essential building blocks still.

## How does it work?
### Running it
Compile and run by specifying flow files to be executed:
```bash
m7m file1.yml file2.yml
```
Optionally, if some files contain multiple flows and you want only a subset, you can specify the flow names you want to run from those files:
```bash
m7m -o myflow,myotherflow file1.yml file2.yml
```

### File format

Here's an example to describe how this works:
```yaml
---
name: mytest
trigger:
    type: timer
    interval: "2s"
notifiers:
    - name: mytelegram
      type: telegram
      token: ABCD
      chat_id: 1234
    - name: printer # for testing
      type: print
steps:
  - get_url:
      output_var: "metrics"
      url: "http://url:port/metrics"
      retries: 2
      retry_interval: "1s"
      upon_failure:
      - notify:
          notifier: mytelegram
          message: "could not retrieve metrics, pausing for 1h"
      - sleep:
          duration: "1h"
      - abort_flow
  - text_extract_one_capture:
      input_var: "metrics"
      regex: "status (.)"
      output_var: "status"
      upon_failure:
          - notify:
              notifier: mytelegram
              message: "could not find status in metrics output"
          - abort_flow
  - type: read_from_file
    output_var: mytext
    path: .cargo/config.toml
    retries: 1
    retry_interval: 2s
  - type: debug_state
  - type: set_variable
    output_var: mytext
    input: Hi there
  - type: append_to_file
    path: testfile.txt
    input_var: mytext
    retries: 2
  - compare_var:
      input_var: status
      compare_with: "1"
      compare_for: equality
      if_false:
        - notify:
            notifier: mytelegram
            message: "Status is down!"
        - set_variable:
            output_var: alert
            input: "Outage recorded"
        - append_to_file:
            path: outages.txt
            input_var: alert
        - type: post_url
          url: http://url:port/api/something
          body: "{ description: \" automated entry from monitoring \" }"
      if_true:
        - type: notify
          notifier: printer
          message: Status is not currently down, all is good
```

The different sections of the file are:
- name: for logs and for the `-o` option on the command line
- trigger: what will trigger the flow, can be an interval timer or one-off (`once`)
- a list of notifiers: things that can get sent some text to push somewhere; currently `telegram` or `print` being the main ones; chains of notifiers can be formed so that one notifier alies dispatches messages to several places
- a list of steps; each step has various parameters (for now, see source code) and most of them have "backup steps" as a list under `upon_failure`; when a failure is detected the list of steps will be executed and then the main flow will resume.
- optionally a root-level `upon_failure` list of steps for uncaught exceptions during the main list of steps
