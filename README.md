# ViewU 
ViewU is a lightweight file format viewer based in the terminal. 
Using ratatui for an interactive tui experience when reading and having CLI tools if you're wanting to convert formats from docx $\to$ .md or even just readable text. 

<div align="center"> 
  
  <img width="2556" height="1593" alt="image" src="https://github.com/user-attachments/assets/43e2db15-eaea-4398-836b-ee2aee87d98b" /> 
  
</div> 

## Supported Formats 

As of right now the goal is to support more formats to be viewable but right now we support 
<div align="center"> 
  
  | viewable formats | support | 
  | --- | --- | 
  | DOCX | ✔ |
  | PDF | ✘ | 
  | md | ✔ |
  | csv ? | ✘ | 

</div> 

# How to use ViewU

ViewU has two primary ways of using it via the `view` and `dump` actions.

## `view` action

Using `viewu view file.docx` will attempt to open and read a file based on it's extension based on supported formats (view supported formats to see which ones it will try and read as. You can specify the format you're trying to read using the `--as <format>` flag which will override the assumed extension. 

It's good to note that `viewu` if a format can't be guessed will default to `docx`.

Also by "open" `viewu` opens a tui for you to scroll and read text just as you would normally.

Within the terminal you can see a "themes" option by pressing 't'. viewu comes packaged with a set amount of themes but in the future we'll let you add your own personal themes. (You can do this but in the future it will be a command and let you do it in bulk and make it way nicer).

## `dump` action
`viewu dump file.docx -o here.txt` the `dump` action takes a file reads it in as `viewu` text objects and then will spit out an approximation of the file. `viewu` lets you output to both `text` and `.md` files for free from any support format. 

You can specify the format you're reading in from with `--as <format>` specficy the output format with `--format <format>` and give an output location with `-o <file>` 

# Installation 
You can run the command below to install it directly
```
cargo install viewu
```
