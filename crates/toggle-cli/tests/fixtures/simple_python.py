def main():
    # This is a regular comment
    print("Hello, world!")
    
    # toggle:start ID=debug desc="Debug output"
#   print("Debug information")
#   print("This is some debug code")
#   print("That should be easy to toggle")
    # toggle:end ID=debug
    
    # Another section that can be toggled
    # toggle:start ID=feature desc="Experimental feature"
    print("This is an experimental feature")
    print("It can be toggled on or off")
    # toggle:end ID=feature
    
    print("End of program")

if __name__ == "__main__":
    main() 
