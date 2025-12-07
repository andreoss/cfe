Feature: Joining a board
  Somebody may make an account, and afterwards that account is known
  by its name.

  Scenario: A new account can be made and is then known
    Given the board is answering
    When somebody makes a new account
    Then the account can be looked up by its name
