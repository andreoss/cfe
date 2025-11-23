import { Builder, By, until } from 'selenium-webdriver'
import chrome from 'selenium-webdriver/chrome.js'
import { promoteViaRoot, publishTopic } from './support.mjs'

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

async function openNewTopic(driver) {
  let last
  for (let attempt = 0; attempt < 10; attempt += 1) {
    try {
      await clickWhenReady(
        driver,
        By.xpath("//button[text()='New topic']"),
        'the new topic control should be present',
      )
      await driver.wait(until.elementLocated(By.name('title')), 2000)
      return
    } catch (err) {
      last = err
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

async function signIn(driver, username) {
  await driver.get(`${baseUrl}/sign-in`)
  await driver.wait(until.elementLocated(By.name('username')), 5000)
  await driver.findElement(By.name('username')).sendKeys(username)
  await driver.findElement(By.name('password')).sendKeys('correcthorse')
  await driver.findElement(By.css('button[type="submit"]')).click()
  await driver.wait(until.urlIs(`${baseUrl}/`), 10000)
}

async function signOut(driver) {
  await driver.get(`${baseUrl}/`)
  await clickWhenReady(
    driver,
    By.xpath("//button[normalize-space(.)='Sign out']"),
    'the sign-out control should be present',
  )
  await driver.wait(
    async () => (await driver.findElement(By.css('nav')).getText()).includes('Sign in'),
    10000,
    'signing out should show the sign-in link',
  )
}

async function reportTopic(driver, topicUrl, kind, reason) {
  await driver.get(topicUrl)
  await clickWhenReady(driver, By.xpath("//button[text()='Report']"), 'the report control')
  const select = await driver.wait(until.elementLocated(By.name('report-kind')), 10000)
  await select.findElement(By.xpath(`./option[@value='${kind}']`)).click()
  const input = await driver.findElement(By.name('report-reason'))
  await input.clear()
  await input.sendKeys(reason)
  await clickWhenReady(driver, By.xpath("//button[text()='Send report']"), 'the send control')
  let outcome = ''
  await driver.wait(
    async () => {
      const text = await mainText(driver)
      if (text.includes('Report sent.') || text.includes('already reported')) {
        outcome = text
        return true
      }
      return false
    },
    10000,
    'reporting should either land or be refused',
  )
  return outcome
}

async function run() {
  const mod = await buildDriver()
  const author = await buildDriver()
  const reader = await buildDriver()
  const suffix = Date.now().toString(36)
  const modName = `e2e_rp_m_${suffix}`
  const authorName = `e2e_rp_a_${suffix}`
  const readerName = `e2e_rp_r_${suffix}`
  const topicTitle = `Reportable topic ${suffix}`
  try {
    await register(mod, modName)
    await promoteViaRoot(modName)
    await signIn(mod, modName)
    await register(author, authorName)
    await register(reader, readerName)

    await author.get(`${baseUrl}/s/general`)
    await openNewTopic(author)
    await author.findElement(By.name('title')).sendKeys(topicTitle)
    await author.findElement(By.name('body')).sendKeys('A topic that will be reported.')
    await clickWhenReady(author, By.xpath("//button[text()='Post']"), 'post control')
    await publishTopic(topicTitle)

    await author.get(`${baseUrl}/s/general`)
    await clickWhenReady(author, By.linkText(topicTitle), 'the published topic should be listed')
    await author.wait(until.elementLocated(By.name('comment-body')), 10000)
    const topicUrl = await author.getCurrentUrl()

    let text = await reportTopic(reader, topicUrl, 'rule', `Breaks a rule ${suffix}`)
    assert(text.includes('Report sent.'), `a reader should be able to report, saw: ${text}`)

    text = await reportTopic(reader, topicUrl, 'rule', `Again ${suffix}`)
    assert(
      text.toLowerCase().includes('already reported'),
      `reporting the same topic twice should be refused, saw: ${text}`,
    )

    await signOut(reader)
    await reader.get(topicUrl)
    await reader.wait(until.elementLocated(By.css('main')), 10000)
    const anonymous = await reader.findElements(By.xpath("//button[text()='Report']"))
    assert(anonymous.length === 0, 'an anonymous visitor must not see a report control')

    await reader.get(`${baseUrl}/reports`)
    await reader.wait(
      async () => (await mainText(reader)).includes('Only a moderator can review reports.'),
      10000,
      'a signed-out visitor must be told the queue is for moderators',
    )

    await signIn(author, authorName)
    await author.get(`${baseUrl}/reports`)
    await author.wait(
      async () => (await mainText(author)).includes('Only a moderator can review reports.'),
      10000,
      'a plain user must not read the queue',
    )
    const noLink = await author.findElements(By.linkText('Reports'))
    assert(noLink.length === 0, 'a plain user must not see the reports link')

    await mod.get(`${baseUrl}/reports`)
    await mod.wait(
      async () => (await mainText(mod)).includes(`Breaks a rule ${suffix}`),
      10000,
      'a moderator should see the open report',
    )
    text = await mainText(mod)
    assert(text.includes(readerName), `the queue should name the reporter, saw: ${text}`)
    assert(text.includes('rule'), `the queue should show the kind, saw: ${text}`)

    await mod.get(topicUrl)
    await mod.wait(
      async () => (await mainText(mod)).includes('Open reports: 1'),
      10000,
      'a moderator should see the open report count on the topic',
    )

    await mod.get(`${baseUrl}/reports`)
    await clickWhenReady(mod, By.xpath("//button[text()='Close report']"), 'the close control')
    await mod.wait(
      async () => !(await mainText(mod)).includes(`Breaks a rule ${suffix}`),
      10000,
      'closing should take the report out of the queue',
    )

    await mod.get(topicUrl)
    await mod.wait(
      async () => (await mainText(mod)).includes('Open reports: 0'),
      10000,
      'closing should clear the count on the topic',
    )

    console.log('e2e: content reporting flow passed')
  } finally {
    await mod.quit()
    await author.quit()
    await reader.quit()
  }
}

run().catch((err) => {
  console.error(err)
  process.exit(1)
})
