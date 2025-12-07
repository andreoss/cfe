import { Builder, By, until } from 'selenium-webdriver'
import chrome from 'selenium-webdriver/chrome.js'
import { publishTopic } from './support.mjs'

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

async function clickWhenReady(driver, locator, message) {
  await driver.wait(until.elementLocated(locator), 15000, message)
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
  await driver.wait(until.elementLocated(By.name('username')), 15000)
  await driver.findElement(By.name('username')).sendKeys(username)
  await driver.findElement(By.name('email')).sendKeys(`${username}@example.com`)
  await driver.findElement(By.name('password')).sendKeys('correcthorse')
  await driver.findElement(By.css('button[type="submit"]')).click()
  await driver.wait(until.urlIs(`${baseUrl}/`), 15000)
}

async function startTopic(driver, title, body) {
  await driver.get(`${baseUrl}/s/general`)
  await driver.wait(
    until.elementLocated(By.xpath("//button[text()='New topic']")),
    15000,
    'the new topic control should appear once posting is known to be allowed',
  )
  await driver.findElement(By.xpath("//button[text()='New topic']")).click()
  await driver.wait(until.elementLocated(By.name('title')), 15000)
  await driver.findElement(By.name('title')).sendKeys(title)
  await driver.findElement(By.name('body')).sendKeys(body)
  await clickWhenReady(driver, By.xpath("//button[text()='Post']"), 'the post control')
  await publishTopic(title)
}

async function choose(driver, name, value) {
  const select = await driver.findElement(By.name(name))
  await select.findElement(By.css(`option[value="${value}"]`)).click()
}

async function hitTitles(driver) {
  const found = await driver.findElements(By.css('li.hit'))
  const titles = []
  for (const hit of found) {
    titles.push((await hit.getText()).replace(/\s+/g, ' '))
  }
  return titles
}

async function settled(driver, want) {
  await driver.wait(
    async () => (await driver.findElements(By.css('li.hit'))).length === want,
    15000,
    `expected ${want} results`,
  )
  return hitTitles(driver)
}

async function leading(driver, want, expected, message) {
  let saw = []
  await driver.wait(
    async () => {
      const found = await driver.findElements(By.css('li.hit'))
      if (found.length !== want) return false
      saw = await hitTitles(driver)
      return saw.length > 0 && saw[0].includes(expected)
    },
    15000,
    () => `${message}, saw: ${saw.join(' / ')}`,
  )
  return saw
}

async function run() {
  const driver = await buildDriver()
  const suffix = Date.now().toString(36)
  const term = `quagga${suffix}`
  const first = `A ${term} in the title`
  const second = `A later subject ${suffix}`
  try {
    await register(driver, `e2e_srch_${suffix}`)

    await startTopic(driver, first, 'A body without the term in it.')
    await startTopic(driver, second, `A body that mentions ${term} in passing.`)

    await driver.get(`${baseUrl}/t/`)
    await driver.get(`${baseUrl}/s/general`)
    await driver.wait(until.elementLocated(By.linkText(second)), 15000)
    await driver.findElement(By.linkText(second)).click()
    await driver.wait(until.elementLocated(By.name('comment-body')), 15000)
    await driver.findElement(By.name('comment-body')).sendKeys(`A remark about ${term} too.`)
    await clickWhenReady(driver, By.xpath("//button[text()='Post comment']"), 'the comment control')
    await driver.wait(
      async () => (await driver.findElement(By.css('main')).getText()).includes('remark about'),
      15000,
      'the remark should be posted before it can be searched for',
    )

    await driver.get(`${baseUrl}/search?q=${term}`)
    const everything = await settled(driver, 3)
    assert(
      everything.some((hit) => hit.includes('Comment by')),
      `everything should include the remark, saw: ${everything.join(' / ')}`,
    )

    await choose(driver, 'scope', 'topics')
    const topicsOnly = await settled(driver, 2)
    assert(
      !topicsOnly.some((hit) => hit.includes('Comment by')),
      `topics should leave the remark out, saw: ${topicsOnly.join(' / ')}`,
    )
    await driver.wait(
      async () => (await driver.getCurrentUrl()).includes('scope=topics'),
      15000,
      'the narrowing should be kept in the address',
    )

    await choose(driver, 'scope', 'comments')
    const commentsOnly = await settled(driver, 1)
    assert(
      commentsOnly[0].includes('Comment by'),
      `comments should leave the subjects out, saw: ${commentsOnly.join(' / ')}`,
    )

    await choose(driver, 'scope', 'topics')
    await settled(driver, 2)
    await choose(driver, 'order', 'newest')
    await leading(driver, 2, second, 'newest should put the later subject first')

    await choose(driver, 'order', 'oldest')
    await leading(driver, 2, first, 'oldest should be the other way round')

    const shared = await driver.getCurrentUrl()
    assert(shared.includes('order=oldest'), `the order should be in the address, was: ${shared}`)
    await driver.get(shared)
    await leading(driver, 2, first, 'a shared search should come back the same')
    const kept = await driver.findElement(By.name('order')).getAttribute('value')
    assert(kept === 'oldest', `the control should show what was asked for, showed: ${kept}`)

    console.log('e2e: search filters flow passed')
  } finally {
    await driver.quit()
  }
}

run().catch((err) => {
  console.error(err)
  process.exit(1)
})
