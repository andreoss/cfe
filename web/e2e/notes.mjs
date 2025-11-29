import { Builder, By, until } from 'selenium-webdriver'
import chrome from 'selenium-webdriver/chrome.js'

const baseUrl = process.env.BASE_URL ?? 'http://127.0.0.1:58081'
const chromedriverPath = process.env.CHROMEDRIVER_PATH

function assert(condition, message) {
  if (!condition) throw new Error(message)
}

async function buildDriver() {
  const options = new chrome.Options()
  options.addArguments('--headless=new', '--no-sandbox', '--disable-gpu')
  const builder = new Builder().forBrowser('chrome').setChromeOptions(options)
  if (chromedriverPath) {
    const service = new chrome.ServiceBuilder(chromedriverPath)
    builder.setChromeService(service)
  }
  return builder.build()
}

async function mainText(driver) {
  let last
  for (let attempt = 0; attempt < 20; attempt += 1) {
    try {
      return await driver.findElement(By.css('main')).getText()
    } catch (err) {
      last = err
      await new Promise((resolve) => setTimeout(resolve, 250))
    }
  }
  throw last
}

async function clickWhenReady(driver, locator, message) {
  await driver.wait(until.elementLocated(locator), 10000, message)
  let last
  for (let attempt = 0; attempt < 20; attempt += 1) {
    try {
      await driver.findElement(locator).click()
      return
    } catch (err) {
      last = err
      await new Promise((resolve) => setTimeout(resolve, 250))
    }
  }
  throw last
}

async function register(driver, username) {
  await driver.get(`${baseUrl}/register`)
  await driver.wait(until.elementLocated(By.name('username')), 5000)
  await driver.findElement(By.name('username')).sendKeys(username)
  await driver.findElement(By.name('email')).sendKeys(`${username}@example.com`)
  await driver.findElement(By.name('password')).sendKeys('correcthorse')
  await driver.findElement(By.css('button[type="submit"]')).click()
  await driver.wait(until.urlIs(`${baseUrl}/`), 10000)
}

async function writeNote(driver, username, text) {
  await driver.get(`${baseUrl}/u/${username}`)
  const box = await driver.wait(until.elementLocated(By.name('remark-text')), 10000)
  await box.clear()
  await box.sendKeys(text)
  await clickWhenReady(driver, By.xpath("//button[text()='Save note']"), 'the save note control')
  await driver.wait(
    async () => {
      const text = await mainText(driver)
      if (text.includes('Note saved.')) return true
      return (await driver.findElements(By.css('[role="alert"]'))).length > 0
    },
    10000,
    'saving a note should report an outcome',
  )
  const outcome = await mainText(driver)
  if (!outcome.includes('Note saved.')) {
    throw new Error(`saving a note failed: ${outcome}`)
  }
}

async function run() {
  const author = await buildDriver()
  const other = await buildDriver()
  const subject = await buildDriver()
  const suffix = Date.now().toString(36)
  const authorName = `e2e_nt_a_${suffix}`
  const otherName = `e2e_nt_o_${suffix}`
  const subjectName = `e2e_nt_s_${suffix}`
  try {
    await register(author, authorName)
    await register(other, otherName)
    await register(subject, subjectName)

    await author.get(`${baseUrl}/notes`)
    await author.wait(
      async () => (await mainText(author)).includes('You have not written any notes.'),
      10000,
      'the notes list should start empty',
    )

    await author.get(`${baseUrl}/u/${authorName}`)
    await author.wait(until.elementLocated(By.css('main')), 10000)
    const own = await author.findElements(By.name('remark-text'))
    assert(own.length === 0, 'nobody may write a note on their own profile')

    await writeNote(author, subjectName, `Helpful in the tagging thread ${suffix}`)

    await author.get(`${baseUrl}/u/${subjectName}`)
    await author.wait(
      async () => {
        const box = await author.findElements(By.name('remark-text'))
        if (box.length === 0) return false
        return (await box[0].getAttribute('value')).includes(suffix)
      },
      10000,
      'the note should be loaded back for its author',
    )

    await subject.get(`${baseUrl}/u/${subjectName}`)
    await subject.wait(until.elementLocated(By.css('main')), 10000)
    const subjectSees = await mainText(subject)
    assert(
      !subjectSees.includes(`Helpful in the tagging thread ${suffix}`),
      `the subject must never see a note about them, saw: ${subjectSees}`,
    )

    await other.get(`${baseUrl}/u/${subjectName}`)
    await other.wait(until.elementLocated(By.name('remark-text')), 10000)
    const otherBox = await other.findElement(By.name('remark-text'))
    assert(
      (await otherBox.getAttribute('value')) === '',
      'another reader must not see somebody elses note',
    )
    await writeNote(other, subjectName, `A different opinion ${suffix}`)

    await author.get(`${baseUrl}/notes`)
    await author.wait(
      async () => (await mainText(author)).includes(`Helpful in the tagging thread ${suffix}`),
      10000,
      'the notes list should name the note',
    )
    const authorNotes = await mainText(author)
    assert(
      authorNotes.includes(subjectName),
      `the notes list should name the subject, saw: ${authorNotes}`,
    )
    assert(
      !authorNotes.includes(`A different opinion ${suffix}`),
      'one reader must not see another readers note',
    )

    await author.get(`${baseUrl}/u/${subjectName}`)
    await clickWhenReady(author, By.xpath("//button[text()='Clear note']"), 'the clear control')
    await author.wait(
      async () => (await mainText(author)).includes('Note cleared.'),
      10000,
      'clearing should report success',
    )

    await author.get(`${baseUrl}/notes`)
    await author.wait(
      async () => (await mainText(author)).includes('You have not written any notes.'),
      10000,
      'the notes list should be empty again',
    )

    await other.get(`${baseUrl}/notes`)
    await other.wait(
      async () => (await mainText(other)).includes(`A different opinion ${suffix}`),
      10000,
      'clearing one note must not clear another readers',
    )

    console.log('e2e: private notes flow passed')
  } finally {
    await author.quit()
    await other.quit()
    await subject.quit()
  }
}

run().catch((err) => {
  console.error(err)
  process.exit(1)
})
