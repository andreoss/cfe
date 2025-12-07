Feature: Reading a board
  Anybody may look at a board without an account: what it is divided
  into, what has been written, and who wrote it.

  Scenario: A board is divided into places to post
    Given the board is answering
    Then it offers at least one section

  Scenario: A section can be opened by anybody
    Given the board is answering
    When somebody who has not signed in opens a section
    Then the section names itself

  Scenario: A subject can be read by anybody
    Given the board is answering
    When somebody who has not signed in opens a subject
    Then the subject shows its title
    And the subject names who wrote it

  Scenario: An account can be looked up by anybody
    Given the board is answering
    When somebody looks up the account of an administrator
    Then the account names itself

  Scenario: A subject that is not there is not invented
    Given the board is answering
    When somebody opens a subject that does not exist
    Then the board says there is nothing there
